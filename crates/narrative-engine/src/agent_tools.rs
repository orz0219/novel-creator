//! P2 真实领域工具（组合根实现）
//!
//! 智能体的领域级 Action 实现，直接调用 `application` 层 service，并经
//! `MutationCommitter` 提交（统一 Canon 写路径）。Phase A 覆盖 Entity 聚合，
//! Phase B 覆盖 Narrative / Storyline / Foreshadow / Rule / Snapshot / Project /
//! World / History 聚合。
//!
//! 依赖方向：本模块位于 `narrative-engine`（组合根 / host），同时依赖 `agent`
//! （取 `AgentTool` 端口）与 `application`（取 service），符合依赖倒置约定：
//! 工具实现不在 `agent` crate 内，而在组合根构造并注入 `ToolRegistry`。
//!
//! 工具命名即领域动作；删除类统一走 service 的 delete_*（其中 Entity 为软删除
//! status='Deleted'、Relation 为语义化结束 valid_until、Narrative 为软删除），
//! 历史 Event / Fact 仅提供创建与读取，不提供修改 / 删除（不可篡改）。

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use agent::{AgentTool, ToolRegistry};
use anyhow::Result;
use application::entity_service::EntityService;
use application::foreshadow_service::ForeshadowService;
use application::history_service::HistoryService;
use application::mutation::MutationCommitter;
use application::narrative_service::NarrativeService;
use application::project_service::ProjectService;
use application::rule_service::RuleService;
use application::snapshot_service::SnapshotService;
use application::storyline_service::StorylineService;
use application::world_service::WorldService;
use async_trait::async_trait;
use db::application_ports::{
    DbEntityRepositoryPort, DbForeshadowRepositoryPort, DbHistoryRepositoryPort, DbNarrativeRepositoryPort,
    DbNarrativeStateWritePort, DbProjectRepositoryPort, DbRuleRepositoryPort, DbSnapshotRepositoryPort,
    DbStorylineRepositoryPort, DbWorldRepositoryPort,
};
use db::mutation_committer::DbMutationCommitter;
use db::project_resolver::DbProjectResolverPort;
use domain::world::World;
use domain::{foreshadowing as fs_domain, storyline as sl_domain};
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

// ============================================================
// 通用辅助
// ============================================================

fn parse_uuid(v: &Value, key: &str) -> Result<Uuid> {
    v.get(key)
        .and_then(|x| x.as_str())
        .and_then(|s| Uuid::parse_str(s).ok())
        .ok_or_else(|| anyhow::anyhow!("字段 '{}' 缺失或不是合法 UUID", key))
}

fn opt_str<'a>(v: &'a Value, key: &str) -> Option<&'a str> {
    v.get(key).and_then(|x| x.as_str())
}

/// 取可选字符串入参，返回 owned 值：缺失 / null / 空串 → `None`（表示不改）。
///
/// 与 [`opt_str`] 的差别只在生命周期：`revise_*` 需要把值传进
/// 「`Option<&str>` 表示保持原值」的服务方法，借用无法跨多个字段同时存活，
/// 因此这里返回 `String`。
fn opt_str_owned(v: &Value, key: &str) -> Option<String> {
    v.get(key)
        .and_then(|x| x.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

/// 解析**枚举型字符串入参**：把中西文写法归一到 DB 存的英文值。
///
/// 存在的意义是「脏数据在这里就拦住」。此前工具层把 `importance` / `hint_level`
/// 之类的字符串直接写库，既不校验取值、默认值还给过非法值（伏笔的 `"low"`
/// 就不在合法集合里），结果是前端展示、按状态筛选全部失准。
///
/// `parse` 由 domain 的枚举提供（含中文别名），`allowed` 是同一枚举的取值清单——
/// 两者都来自 domain 常量，因此错误提示里列出的合法值不可能与校验逻辑漂移。
fn parse_enum_arg(
    raw: &str,
    allowed: &[&str],
    parse: impl Fn(&str) -> Option<&'static str>,
    field: &str,
) -> Result<&'static str> {
    parse(raw).ok_or_else(|| {
        anyhow::anyhow!(
            "{} 无法识别：{}（合法取值：{}）",
            field,
            raw,
            allowed.join(" / ")
        )
    })
}

/// 取可选枚举入参：缺失 / null / 空串 → `None`（表示不改），非法值直接报错。
/// 取可选整数（era_order 这类排序锚）：非整数直接报错，不静默忽略。
fn opt_i64_strict(v: &Value, key: &str) -> Result<Option<i64>> {
    match v.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Number(n)) => n
            .as_i64()
            .map(Some)
            .ok_or_else(|| anyhow::anyhow!("{} 应为整数，收到：{}", key, n)),
        Some(other) => anyhow::bail!("{} 应为整数，收到：{}", key, other),
    }
}

fn opt_enum_arg(
    v: &Value,
    key: &str,
    parse: impl Fn(&str) -> Option<&'static str>,
    allowed: &[&str],
) -> Result<Option<&'static str>> {
    let Some(raw) = v.get(key) else {
        return Ok(None);
    };
    if raw.is_null() {
        return Ok(None);
    }
    let s = raw
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("{} 应为字符串，收到：{}", key, raw))?;
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    parse_enum_arg(trimmed, allowed, parse, key).map(Some)
}

/// 取可选 UUID，但**不吞掉非法值**。
///
/// 与 [`opt_uuid`] 的差异在「把伏笔挂到某条剧情线」这种语义下是必要的：那个版本把
/// 「写了但不是 UUID」和「压根没写」都变成 `None`，模型传个错 id 就会**静默地什么都不挂**，
/// 却以为挂上了。这里：缺字段 / 空串返回 `None`，写了非法值直接报错。
fn opt_uuid_strict(v: &Value, key: &str) -> Result<Option<Uuid>> {
    match v.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => {
            let trimmed = s.trim();
            if trimmed.is_empty() {
                return Ok(None);
            }
            Uuid::parse_str(trimmed)
                .map(Some)
                .map_err(|_| anyhow::anyhow!("{} 不是合法 UUID：{}", key, s))
        }
        Some(other) => anyhow::bail!("{} 应为 UUID 字符串，收到：{}", key, other),
    }
}

/// 框架在工具执行前会注入的字段（见 `agent::inject_project_id`：用会话的
/// project_id 覆盖模型传入值以实现物理隔离）。
///
/// 它们不属于任何工具的入参契约，因此对入参做严格校验的工具必须放行它们，
/// 否则会被误判成"未知字段"。`project_id` 的字段名只有一处真相（`agent` crate），
/// 这里引用常量而不是再写一遍字面量。
const FRAMEWORK_INJECTED_FIELDS: [&str; 2] = [agent::PROJECT_ID_FIELD, "world_id"];

/// 把 `get_*` 读到的档案拆成「可写字段」与「只读字段」两部分。
///
/// 让读写两侧的字段集在**返回值里就看得出来**：模型读到 `readonly` 里的内容时，
/// 自然不会把它当作可写字段回传；即使回传了，写入工具也会忽略并回执。
fn split_readonly(value: &Value, readonly_keys: &[&str]) -> (Value, Value) {
    let mut data = serde_json::Map::new();
    let mut readonly = serde_json::Map::new();
    if let Some(obj) = value.as_object() {
        for (k, v) in obj {
            if readonly_keys.contains(&k.as_str()) {
                readonly.insert(k.clone(), v.clone());
            } else {
                data.insert(k.clone(), v.clone());
            }
        }
    }
    (Value::Object(data), Value::Object(readonly))
}

// ============================================================
// Entity 聚合（Phase A）
// ============================================================

#[derive(Clone, Copy)]
pub enum EntityAction {
    CreateCharacter,
    CreateLocation,
    CreateFaction,
    CreateItem,
    /// 通用创建：类型由入参指定（用于 golden_finger 等没有专用工具的类型）
    CreateEntity,
    ReviseEntity,
    RetireEntity,
    CreateRelation,
    EndRelation,
    /// 改关系属性：没有它，描述写错只能结束旧边 + 重建（id 会变）
    ReviseRelation,
    /// 读取关系边：没有它就无法回答「谁和谁有关系」，也拿不到 end_relation 需要的 id
    ListRelations,
    GetEntity,
    ListEntities,
    /// 实体类型清单：类型名只存在于 Rust 的 match 分支里，不暴露就只能靠猜
    ListEntityTypes,
}

pub struct EntityTool {
    action: EntityAction,
    service: Arc<EntityService>,
}

impl EntityTool {
    pub fn new(action: EntityAction, service: Arc<EntityService>) -> Self {
        Self { action, service }
    }
}

/// 构造并注册所有 Entity 领域工具到 `registry`。
pub fn register_entity_tools(registry: &ToolRegistry, service: Arc<EntityService>) {
    let actions = [
        EntityAction::CreateCharacter,
        EntityAction::CreateLocation,
        EntityAction::CreateFaction,
        EntityAction::CreateItem,
        EntityAction::CreateEntity,
        EntityAction::ReviseEntity,
        EntityAction::RetireEntity,
        EntityAction::CreateRelation,
        EntityAction::EndRelation,
        EntityAction::ReviseRelation,
        EntityAction::ListRelations,
        EntityAction::GetEntity,
        EntityAction::ListEntities,
        EntityAction::ListEntityTypes,
    ];
    for a in actions {
        registry.register(Arc::new(EntityTool::new(a, service.clone())));
    }
}

fn entity_name(a: EntityAction) -> &'static str {
    match a {
        EntityAction::CreateCharacter => "create_character",
        EntityAction::CreateLocation => "create_location",
        EntityAction::CreateFaction => "create_faction",
        EntityAction::CreateItem => "create_item",
        EntityAction::CreateEntity => "create_entity",
        EntityAction::ReviseEntity => "revise_entity",
        EntityAction::RetireEntity => "retire_entity",
        EntityAction::CreateRelation => "create_relation",
        EntityAction::EndRelation => "end_relation",
        EntityAction::ReviseRelation => "revise_relation",
        EntityAction::ListRelations => "list_relations",
        EntityAction::GetEntity => "get_entity",
        EntityAction::ListEntities => "list_entities",
        EntityAction::ListEntityTypes => "list_entity_types",
    }
}

fn entity_description(a: EntityAction) -> String {
    match a {
        EntityAction::CreateCharacter => "创建角色（Character）实体并落库（经 Canon 写路径）。".into(),
        EntityAction::CreateLocation => "创建地点（Location）实体并落库。".into(),
        EntityAction::CreateFaction => "创建势力（Faction）实体并落库。".into(),
        EntityAction::CreateItem => "创建物品（Item）实体并落库。".into(),
        EntityAction::CreateEntity => {
            "按指定类型创建实体并落库。用于没有专用工具的类型，例如金手指（entity_type='golden_finger'）。\
             常用类型：Character / Location / Faction / Item / golden_finger / Creature / Organization / Event。"
                .into()
        }
        EntityAction::ReviseEntity => {
            "修改已有实体的名称 / 简介 / 描述 / 属性（仅传入字段会被更新）。这会修改已有产物。".into()
        }
        EntityAction::RetireEntity => {
            "逻辑删除（语义化软删除，绝不物理 DELETE）一个实体。删除前请先用 get_entity 确认目标 id。".into()
        }
        EntityAction::CreateRelation => {
            "在两个实体间创建关系（relation）并落库。relation_type 请优先使用中文，\
             例如：敌对 / 控制 / 包含 / 位于 / 拥有 / 隶属 / 涉及 / 夺取生命 / 朋友 / 盟友 / 师从 / 创立。"
                .into()
        }
        EntityAction::EndRelation => {
            "语义化结束一条关系（**等同 retire_relation**，同一操作的不同叫法），\
             保留历史边但不生效，不是物理删除。\
             必须先从 list_relations 取到该关系的 id——本工具只接受 id。"
                .into()
        }
        EntityAction::ReviseRelation => {
            "修改已有关系的关系类型 / 描述（只传要改的字段，其余保持原值；两端实体不变）。\
             关系描述写错时用它改，**不要**结束旧边再重建——那样 id 会变、时间线会断成两段。\
             必须先用 list_relations 取到该关系的 id。"
                .into()
        }
        EntityAction::ListRelations => {
            "读取关系边（谁和谁有关系）。传 world_id 列出该世界全部关系；\
             传 entity_id 只列出与该实体相关的关系（两端都算）。\
             返回值含 id / source_name / target_name / relation_type / description，\
             其中 id 是 end_relation 与查重所需的键。创建关系前应先查一次，避免重复建边。"
                .into()
        }
        EntityAction::GetEntity => {
            "读取单一实体的当前状态（含版本号），修改 / 删除前应先调用以确认目标。\
             注意：实体详情**不含关系边**，要查某个实体与谁有关请用 list_relations。"
                .into()
        }
        EntityAction::ListEntities => {
            "列出某世界下的实体（**目录页**：只含 id / 名称 / 类型 id / 一句话摘要，不含正文）。\
             **entity_type 可省略**：省略即返回该世界全部类型的实体（跨类型检索用，例如\
             「找出所有和破庙相关的实体」）；传入则只返回该类型，可用类型见 list_entity_types。\
             默认返回 20 条并给出 total；用 limit / offset 翻页，**不要**指望一次拿全量。\
             需要某个实体的正文 / 档案，用 get_entity 或 get_character_profile 等按需读取。"
                .into()
        }
        EntityAction::ListEntityTypes => {
            "列出系统支持的全部实体类型（含可用的中文别名）。\
             在传 entity_type / create_entity 之前先用它确认合法取值，不要凭印象猜类型名。"
                .into()
        }
    }
}

fn entity_schema(a: EntityAction) -> Value {
    match a {
        EntityAction::CreateCharacter
        | EntityAction::CreateLocation
        | EntityAction::CreateFaction
        | EntityAction::CreateItem => json!({
            "type": "object",
            "properties": {
                "world_id": { "type": "string", "description": "目标世界 UUID（先调 get_main_world 取得）" },
                "name": { "type": "string" },
                "summary": { "type": "string" },
                "description": { "type": "string" }
            },
            "required": ["world_id", "name"]
        }),
        EntityAction::CreateEntity => json!({
            "type": "object",
            "properties": {
                "world_id": { "type": "string", "description": "目标世界 UUID（先调 get_main_world 取得）" },
                "entity_type": {
                    "type": "string",
                    "description": "实体类型。可传中文「人物」「地点」「势力」「物品」「组织」「生物」「事件」「神明」「金手指」，或英文 Character / Location / Faction / Item / Organization / Creature / Event / Deity / golden_finger"
                },
                "name": { "type": "string" },
                "summary": { "type": "string" },
                "description": { "type": "string" }
            },
            "required": ["world_id", "entity_type", "name"]
        }),
        EntityAction::ReviseEntity => json!({
            "type": "object",
            "properties": {
                "id": { "type": "string", "description": "要修改的实体 UUID" },
                "name": { "type": "string" },
                "summary": { "type": "string" },
                "description": { "type": "string" },
                "attributes": { "type": "object", "description": "实体自定义属性（JSON 对象）" },
                "append": {
                    "type": "boolean",
                    "description": "为 true 时把本次的 summary / description **追加**到原值后面（默认 false = 覆盖）。长描述分次写时用它，不必每次重发整段。"
                }
            },
            "required": ["id"]
        }),
        EntityAction::RetireEntity | EntityAction::EndRelation | EntityAction::GetEntity => json!({
            "type": "object",
            "properties": { "id": { "type": "string", "description": "目标 UUID" } },
            "required": ["id"]
        }),
        EntityAction::ReviseRelation => json!({
            "type": "object",
            "properties": {
                "id": { "type": "string", "description": "关系边 UUID（用 list_relations 查）" },
                "relation_type": { "type": "string", "description": "新的关系类型；不传则不改" },
                "description": { "type": "string", "description": "新的关系描述；不传则不改" }
            },
            "required": ["id"]
        }),
        EntityAction::CreateRelation => json!({
            "type": "object",
            "properties": {
                "source_entity_id": { "type": "string" },
                "target_entity_id": { "type": "string" },
                "relation_type": { "type": "string", "description": "关系类型（中文优先）：敌对 / 控制 / 包含 / 位于 / 拥有 / 隶属 / 涉及 / 夺取生命 / 朋友 / 盟友 / 师从 / 创立 等" },
                "description": { "type": "string" }
            },
            "required": ["source_entity_id", "target_entity_id", "relation_type"]
        }),
        EntityAction::ListEntities => json!({
            "type": "object",
            "properties": {
                "world_id": { "type": "string", "description": "世界 UUID" },
                "entity_type": { "type": "string", "description": "**可选**：不传则返回该世界全部类型的实体。可传中文「人物」「地点」「势力」「物品」「金手指」，或英文 Character / Location / Faction / Item …（完整清单见 list_entity_types）" },
                "limit": { "type": "number", "description": format!("**可选**：本页条数，默认 {}，上限 {}（超过报错）。列表只给目录字段（id / 名称 / 类型 id / 摘要），正文用 get_entity 或 get_*_profile 取", agent::LIST_DEFAULT_LIMIT, agent::LIST_MAX_LIMIT) },
                "offset": { "type": "number", "description": "**可选**：从第几条开始（默认 0）。返回里的 next_offset 就是下一页该传的值" }
            },
            "required": ["world_id"]
        }),
        EntityAction::ListRelations => json!({
            "type": "object",
            "properties": {
                "world_id": { "type": "string", "description": "世界 UUID：列出该世界全部关系（二选一）" },
                "entity_id": { "type": "string", "description": "实体 UUID：只列出与该实体相关的关系（二选一，两端都算）" }
            },
            "required": []
        }),
        EntityAction::ListEntityTypes => json!({
            "type": "object",
            "properties": {},
            "required": []
        }),
    }
}

#[async_trait]
impl AgentTool for EntityTool {
    fn name(&self) -> String {
        entity_name(self.action).to_string()
    }
    fn description(&self) -> String {
        entity_description(self.action)
    }
    fn input_schema(&self) -> Value {
        entity_schema(self.action)
    }

    async fn execute(&self, input: Value) -> Result<Value> {
        match self.action {
            EntityAction::CreateCharacter => {
                let world_id = parse_uuid(&input, "world_id")?;
                let name = opt_str(&input, "name").ok_or_else(|| anyhow::anyhow!("name 缺失"))?;
                let e = self
                    .service
                    .create_entity(world_id, "Character", name, opt_str(&input, "summary"), opt_str(&input, "description"))
                    .await?;
                Ok(json!({ "ok": true, "action": "create_character", "data": e }))
            }
            EntityAction::CreateLocation => {
                let world_id = parse_uuid(&input, "world_id")?;
                let name = opt_str(&input, "name").ok_or_else(|| anyhow::anyhow!("name 缺失"))?;
                let e = self
                    .service
                    .create_entity(world_id, "Location", name, opt_str(&input, "summary"), opt_str(&input, "description"))
                    .await?;
                Ok(json!({ "ok": true, "action": "create_location", "data": e }))
            }
            EntityAction::CreateFaction => {
                let world_id = parse_uuid(&input, "world_id")?;
                let name = opt_str(&input, "name").ok_or_else(|| anyhow::anyhow!("name 缺失"))?;
                let e = self
                    .service
                    .create_entity(world_id, "Faction", name, opt_str(&input, "summary"), opt_str(&input, "description"))
                    .await?;
                Ok(json!({ "ok": true, "action": "create_faction", "data": e }))
            }
            EntityAction::CreateItem => {
                let world_id = parse_uuid(&input, "world_id")?;
                let name = opt_str(&input, "name").ok_or_else(|| anyhow::anyhow!("name 缺失"))?;
                let e = self
                    .service
                    .create_entity(world_id, "Item", name, opt_str(&input, "summary"), opt_str(&input, "description"))
                    .await?;
                Ok(json!({ "ok": true, "action": "create_item", "data": e }))
            }
            EntityAction::CreateEntity => {
                let world_id = parse_uuid(&input, "world_id")?;
                let raw_type = input
                    .get("entity_type")
                    .and_then(|v| v.as_str())
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .ok_or_else(|| anyhow::anyhow!("create_entity 需要 entity_type"))?;
                let entity_type = parse_entity_type(raw_type)?;
                let name = input
                    .get("name")
                    .and_then(|v| v.as_str())
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .ok_or_else(|| anyhow::anyhow!("create_entity 需要 name"))?;
                let e = self
                    .service
                    .create_entity(
                        world_id,
                        entity_type,
                        name,
                        opt_str(&input, "summary"),
                        opt_str(&input, "description"),
                    )
                    .await?;
                Ok(json!({ "ok": true, "action": "create_entity", "data": e }))
            }
            EntityAction::ReviseEntity => {
                let id = parse_uuid(&input, "id")?;
                // append=true：summary / description 追加到原值后面，而不是覆盖。
                // 长文本常被单次输出上限截断，分次写作时不必把整段重发一遍。
                let append = input
                    .get("append")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let mut summary: Option<String> = opt_str(&input, "summary").map(str::to_string);
                let mut description: Option<String> =
                    opt_str(&input, "description").map(str::to_string);
                if append {
                    let existing = self
                        .service
                        .get_entity(id)
                        .await?
                        .ok_or_else(|| anyhow::anyhow!("实体不存在: {}", id))?;
                    let old_summary = existing
                        .get("summary")
                        .and_then(|v| v.as_str())
                        .map(str::to_string);
                    let old_description = existing
                        .get("description")
                        .and_then(|v| v.as_str())
                        .map(str::to_string);
                    summary = summary
                        .map(|s| agent::append_or_replace(old_summary.as_deref(), &s, true));
                    description = description
                        .map(|s| agent::append_or_replace(old_description.as_deref(), &s, true));
                }
                let e = self
                    .service
                    .update_entity(id, opt_str(&input, "name"), summary.as_deref(), description.as_deref(), input.get("attributes"))
                    .await?;
                Ok(json!({ "ok": true, "action": "revise_entity", "data": e }))
            }
            EntityAction::RetireEntity => {
                let id = parse_uuid(&input, "id")?;
                let e = self.service.delete_entity(id).await?;
                Ok(json!({ "ok": true, "action": "retire_entity", "data": e }))
            }
            EntityAction::CreateRelation => {
                let source = parse_uuid(&input, "source_entity_id")?;
                let target = parse_uuid(&input, "target_entity_id")?;
                let rtype = opt_str(&input, "relation_type").ok_or_else(|| anyhow::anyhow!("relation_type 缺失"))?;
                let r = self.service.create_relation(source, target, rtype, opt_str(&input, "description")).await?;
                Ok(json!({ "ok": true, "action": "create_relation", "data": r }))
            }
            EntityAction::EndRelation => {
                let id = parse_uuid(&input, "id")?;
                self.service.delete_relation(id).await?;
                Ok(json!({ "ok": true, "action": "end_relation", "id": id.to_string() }))
            }
            EntityAction::ReviseRelation => {
                let id = parse_uuid(&input, "id")?;
                let relation_type = opt_str(&input, "relation_type");
                let description = opt_str(&input, "description");
                self.service
                    .revise_relation(id, relation_type.as_deref(), description.as_deref())
                    .await?;
                Ok(json!({
                    "ok": true,
                    "action": "revise_relation",
                    "id": id.to_string(),
                    "revised": {
                        "relation_type": relation_type,
                        "description": description
                    }
                }))
            }
            EntityAction::ListRelations => {
                // 两种入参二选一：world_id 列全部，entity_id 只列与该实体相关的
                let entity_id = opt_str(&input, "entity_id");
                let world_id = match (opt_str(&input, "world_id"), entity_id.as_deref()) {
                    (Some(_), _) => parse_uuid(&input, "world_id")?,
                    // 只给了 entity_id：由它反查所属世界
                    (None, Some(e)) => {
                        let eid = Uuid::parse_str(e)
                            .map_err(|_| anyhow::anyhow!("entity_id 不是合法 UUID：{}", e))?;
                        let entity = self
                            .service
                            .get_entity(eid)
                            .await?
                            .ok_or_else(|| anyhow::anyhow!("实体不存在: {}", e))?;
                        parse_uuid(&entity, "world_id")?
                    }
                    // 两个都没给：无法确定范围，直接报错而不是悄悄返回空列表
                    (None, None) => {
                        anyhow::bail!("list_relations 需要 world_id 或 entity_id 之一");
                    }
                };

                let (limit, offset) = agent::parse_page_args(&input)?;
                let all = self.service.list_relations(world_id).await?;
                let list: Vec<Value> = match &entity_id {
                    Some(e) => {
                        let target = Uuid::parse_str(e)
                            .map_err(|_| anyhow::anyhow!("entity_id 不是合法 UUID：{}", e))?;
                        let target = target.to_string();
                        all.into_iter()
                            .filter(|r| {
                                r.get("source_entity_id").and_then(|v| v.as_str()) == Some(&target)
                                    || r.get("target_entity_id").and_then(|v| v.as_str())
                                        == Some(&target)
                            })
                            .collect()
                    }
                    None => all,
                };
                // 先按 entity_id 过滤，再分页：total 是过滤后的真实数量
                let total = list.len();
                let data: Vec<Value> = list
                    .iter()
                    .skip(offset)
                    .take(limit)
                    .map(|r| {
                        agent::pick_fields(
                            r,
                            &["id", "source_name", "target_name", "relation_type", "description"],
                        )
                    })
                    .collect();
                Ok(agent::list_envelope(
                    "list_relations",
                    data,
                    total,
                    limit,
                    offset,
                    "关系",
                    vec![],
                ))
            }
            EntityAction::ListEntityTypes => Ok(json!({
                "ok": true,
                "action": "list_entity_types",
                "data": entity_types_catalog(),
            })),
            EntityAction::GetEntity => {
                let id = parse_uuid(&input, "id")?;
                let e = self.service.get_entity(id).await?.ok_or_else(|| anyhow::anyhow!("实体不存在: {}", id))?;
                Ok(json!({ "ok": true, "action": "get_entity", "data": e }))
            }
            EntityAction::ListEntities => {
                let world_id = parse_uuid(&input, "world_id")?;
                let type_filter = match opt_str(&input, "entity_type") {
                    Some(t) if !t.trim().is_empty() => Some(parse_entity_type(t)?),
                    _ => None,
                };
                let (limit, offset) = agent::parse_page_args(&input)?;
                let (items, total) = self
                    .service
                    .list_entities_page(world_id, type_filter, limit, offset)
                    .await?;
                // 目录页投影：只回 id / 名称 / 类型 id / 一句话摘要。
                // 实测不投影时 33 个实体 = 36,555 字符（description 占 51%，另有审计列）。
                let data: Vec<Value> = items
                    .iter()
                    .map(|e| agent::pick_fields(e, &["id", "name", "entity_type_id", "summary"]))
                    .collect();
                Ok(agent::list_envelope(
                    "list_entities",
                    data,
                    total,
                    limit,
                    offset,
                    "实体",
                    vec![],
                ))
            }
        }
    }
}

// ============================================================
// Narrative 聚合（细纲：卷 → 弧 → 章 → 场 → 节拍）
// ============================================================

#[derive(Clone, Copy)]
pub enum NarrativeAction {
    CreateNode,
    ReviseNode,
    RemoveNode,
    GetNode,
    ListNodes,
}

pub struct NarrativeTool {
    action: NarrativeAction,
    service: Arc<NarrativeService>,
}

impl NarrativeTool {
    pub fn new(action: NarrativeAction, service: Arc<NarrativeService>) -> Self {
        Self { action, service }
    }
}

pub fn register_narrative_tools(registry: &ToolRegistry, service: Arc<NarrativeService>) {
    for a in [
        NarrativeAction::CreateNode,
        NarrativeAction::ReviseNode,
        NarrativeAction::RemoveNode,
        NarrativeAction::GetNode,
        NarrativeAction::ListNodes,
    ] {
        registry.register(Arc::new(NarrativeTool::new(a, service.clone())));
    }
    // 批量建树：细纲一次要建几十个节点，逐条调用会把一轮对话拆成几十轮往返
    registry.register(Arc::new(BulkCreateNodesTool::new(service)));
}

fn narrative_name(a: NarrativeAction) -> &'static str {
    match a {
        NarrativeAction::CreateNode => "create_node",
        NarrativeAction::ReviseNode => "revise_node",
        NarrativeAction::RemoveNode => "retire_node",
        NarrativeAction::GetNode => "get_node",
        NarrativeAction::ListNodes => "list_nodes",
    }
}

fn narrative_description(a: NarrativeAction) -> String {
    match a {
        NarrativeAction::CreateNode => "创建叙事节点（卷/弧/章/场/节拍，也支持 custom:自有词表）。\
            可指定父节点、同父下的序号、初始状态，并直接挂载它服务的故事线与阶段、\
            在场的角色 / 地点 / 道具、以及预计章数 / 字数 / 故事时间跨度。"
            .into(),
        NarrativeAction::ReviseNode => "修改叙事节点：文本 / 状态，以及**结构与挂载**——\
            换父节点（移动子树）、兄弟重排（序号自动顺移）、改节点类型、挂载或解绑故事线与阶段、\
            挂载在场角色 / 地点 / 道具、补预计章数 / 字数 / 故事时间跨度。\
            这会修改已有产物；改父节点不会改变节点 id，因此伏笔的埋点 / 回收点不会脱钩。"
            .into(),
        NarrativeAction::RemoveNode => "逻辑删除（软删除 status=Deleted）一个叙事节点及其子树。删除前请先用 get_node 确认目标 id。".into(),
        NarrativeAction::GetNode => "读取单一叙事节点（含故事线名与子节点数）；修改 / 删除前应先确认目标。".into(),
        NarrativeAction::ListNodes => "列出某项目的叙事节点：可分页、可按父节点下钻、只看顶层、\
            按节点类型或故事线筛选；每行带 child_count（直接子节点数），便于逐层展开而不必拉全量。"
            .into(),
    }
}

/// `attributes` 只是**补充**：结构与挂载都有专门字段了，别再往这里塞。
fn attributes_arg(input: &Value) -> Result<Value> {
    match input.get("attributes") {
        None | Some(Value::Null) => Ok(json!({})),
        Some(v @ Value::Object(_)) => Ok(v.clone()),
        Some(other) => anyhow::bail!("attributes 应为对象，收到：{}", other),
    }
}

/// `attributes` 的**可选**解析（revise 用）：给了必须是对象，不给就什么都不改。
///
/// 与 create 用的 [`attributes_arg`] 语义不同——create 不给就是空对象，
/// revise 不给则保持原值（`None` = 不动），不能混用。
fn opt_attributes_arg(input: &Value) -> Result<Option<Value>> {
    match input.get("attributes") {
        None | Some(Value::Null) => Ok(None),
        Some(v @ Value::Object(_)) => Ok(Some(v.clone())),
        Some(other) => anyhow::bail!("attributes 应为对象，收到：{}", other),
    }
}

fn opt_bool(v: &Value, key: &str) -> Result<bool> {
    match v.get(key) {
        None | Some(Value::Null) => Ok(false),
        Some(Value::Bool(b)) => Ok(*b),
        Some(other) => anyhow::bail!("{} 应为 true / false，收到：{}", key, other),
    }
}

fn opt_i32_strict(v: &Value, key: &str) -> Result<Option<i32>> {
    match v.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Number(n)) => n
            .as_i64()
            .and_then(|x| i32::try_from(x).ok())
            .map(Some)
            .ok_or_else(|| anyhow::anyhow!("{} 应为 32 位整数，收到：{}", key, n)),
        Some(other) => anyhow::bail!("{} 应为整数，收到：{}", key, other),
    }
}

/// UUID 数组入参。解析失败直接报错，不静默丢弃写错的 id。
fn opt_uuid_array(v: &Value, key: &str) -> Result<Option<Vec<Uuid>>> {
    match v.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Array(items)) => {
            let mut out = Vec::with_capacity(items.len());
            for (i, item) in items.iter().enumerate() {
                let s = item
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("{}[{}] 应为 UUID 字符串，收到：{}", key, i, item))?;
                out.push(
                    Uuid::parse_str(s.trim())
                        .map_err(|_| anyhow::anyhow!("{}[{}] 不是合法 UUID：{}", key, i, s))?,
                );
            }
            Ok(Some(out))
        }
        Some(other) => anyhow::bail!("{} 应为 UUID 数组，收到：{}", key, other),
    }
}

/// `stage_refs`：附加的「线 × 阶段」引用数组。
fn opt_stage_refs(v: &Value, key: &str) -> Result<Option<Vec<domain::narrative::NarrativeStageRef>>> {
    match v.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Array(items)) => {
            let mut out = Vec::with_capacity(items.len());
            for (i, item) in items.iter().enumerate() {
                let sid = item
                    .get("storyline_id")
                    .or_else(|| item.get("storyline"))
                    .and_then(|x| x.as_str())
                    .ok_or_else(|| {
                        anyhow::anyhow!("{}[{}] 缺少 storyline_id（每条阶段引用都必须指明是哪条线）", key, i)
                    })?;
                let storyline_id = Uuid::parse_str(sid.trim()).map_err(|_| {
                    anyhow::anyhow!("{}[{}].storyline_id 不是合法 UUID：{}", key, i, sid)
                })?;
                let arc_stage = item
                    .get("arc_stage")
                    .and_then(|x| x.as_str())
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(str::to_string);
                out.push(domain::narrative::NarrativeStageRef {
                    storyline_id,
                    arc_stage,
                });
            }
            Ok(Some(out))
        }
        Some(other) => anyhow::bail!("{} 应为对象数组，收到：{}", key, other),
    }
}

/// 从入参读出「结构与挂载」补丁（create 与 revise 共用同一套字段名与解析规则）。
fn node_outline_patch_from(input: &Value) -> Result<domain::narrative::NarrativeNodeOutlinePatch> {
    Ok(domain::narrative::NarrativeNodeOutlinePatch {
        node_type: opt_str_owned(input, "node_type"),
        parent_id: opt_uuid_strict(input, "parent_id")?,
        move_to_root: opt_bool(input, "move_to_root")?,
        sort_order: opt_i32_strict(input, "sort_order")?,
        storyline_id: opt_uuid_strict(input, "storyline_id")?,
        clear_storyline: opt_bool(input, "clear_storyline")?,
        arc_stage: opt_str_owned(input, "arc_stage"),
        stage_refs: opt_stage_refs(input, "stage_refs")?,
        participant_entity_ids: opt_uuid_array(input, "participant_entity_ids")?,
        location_id: opt_uuid_strict(input, "location_id")?,
        clear_location: opt_bool(input, "clear_location")?,
        item_ids: opt_uuid_array(input, "item_ids")?,
        estimated_chapters: opt_i32_strict(input, "estimated_chapters")?,
        estimated_words: opt_i32_strict(input, "estimated_words")?,
        story_time: opt_str_owned(input, "story_time"),
    })
}

/// 节点挂载字段的 schema 片段（create / revise / bulk 三处共用一份）。
fn node_outline_props() -> Vec<(&'static str, Value)> {
    vec![
        (
            "storyline_id",
            json!({ "type": "string", "description": "**可选**：这条节点服务的故事线（主挂载）。先用 list_storylines 拿 id" }),
        ),
        (
            "arc_stage",
            json!({ "type": "string", "description": "**可选**：推进到该故事线的哪个阶段（与 storyline.arc_stages[].stage 同名）。要给就必须同时给 storyline_id" }),
        ),
        (
            "stage_refs",
            json!({
                "type": "array",
                "description": "**可选**：附加的「线 × 阶段」引用——一个节点可以同时服务多条线。整体替换（空数组 = 清空）",
                "items": {
                    "type": "object",
                    "properties": {
                        "storyline_id": { "type": "string", "description": "故事线 id（必填）" },
                        "arc_stage": { "type": "string", "description": "**可选**：阶段名" }
                    },
                    "required": ["storyline_id"]
                }
            }),
        ),
        (
            "participant_entity_ids",
            json!({ "type": "array", "items": { "type": "string" }, "description": "**可选**：在场角色的 entity id 数组（整体替换，空数组 = 清空）" }),
        ),
        (
            "location_id",
            json!({ "type": "string", "description": "**可选**：发生地点的 entity id" }),
        ),
        (
            "item_ids",
            json!({ "type": "array", "items": { "type": "string" }, "description": "**可选**：用到的道具 entity id 数组（整体替换）" }),
        ),
        (
            "estimated_chapters",
            json!({ "type": "number", "description": "**可选**：预计章数" }),
        ),
        (
            "estimated_words",
            json!({ "type": "number", "description": "**可选**：预计字数" }),
        ),
        (
            "story_time",
            json!({ "type": "string", "description": "**可选**：故事内时间跨度（自由文本，例如「第三天黄昏」）" }),
        ),
    ]
}

fn insert_props(target: &mut Value, props: Vec<(&'static str, Value)>) {
    if let Some(map) = target.as_object_mut() {
        for (k, v) in props {
            map.insert(k.to_string(), v);
        }
    }
}

fn narrative_schema(a: NarrativeAction) -> Value {
    match a {
        NarrativeAction::CreateNode => {
            let mut props = json!({
                "node_type": { "type": "string", "description": format!("节点类型。{}", domain::narrative::NarrativeNodeType::legal_values_hint()) },
                "parent_id": { "type": "string", "description": "**可选**：父节点 UUID。不给就是顶层节点（卷通常不带父节点）" },
                "title": { "type": "string" },
                "description": { "type": "string" },
                "content": { "type": "string", "description": "章节 / 节点正文（可选）。长正文被输出上限截断时，可先建节点再用 revise_node + append:true 分次续写。" },
                "attributes": { "type": "object", "description": "补充属性（自由对象）。结构与挂载都有专门字段了，这里只放本工具没有对应字段的补充信息" },
                "sort_order": { "type": "number", "description": "**可选**：同父下的序号（1 起）。给了就插到这个位置、后面的兄弟自动顺移；不给就追加到同父末尾" },
                "status": { "type": "string", "description": "**可选**：初始状态（Draft/Planned/InProgress/Completed/Archived，也接受中文）；不给就是 Draft" }
            });
            insert_props(&mut props, node_outline_props());
            json!({ "type": "object", "properties": props, "required": ["node_type", "title"] })
        }
        NarrativeAction::ReviseNode => {
            let mut props = json!({
                "id": { "type": "string" },
                "title": { "type": "string" },
                "description": { "type": "string" },
                "content": { "type": "string" },
                "status": { "type": "string", "description": "**可选**：新状态（Draft/Planned/InProgress/Completed/Archived，也接受中文）" },
                "append": {
                    "type": "boolean",
                    "description": "为 true 时把本次的 description / content **追加**到原值后面（默认 false = 覆盖）。长章节分次写时用它。"
                },
                "node_type": { "type": "string", "description": format!("**可选**：改节点类型。{}", domain::narrative::NarrativeNodeType::legal_values_hint()) },
                "parent_id": { "type": "string", "description": "**可选**：换到新的父节点（整个子树跟着移动，节点 id 不变）。与 move_to_root 互斥" },
                "move_to_root": { "type": "boolean", "description": "**可选**：移到顶层（parent_id 置空）。与 parent_id 互斥" },
                "sort_order": { "type": "number", "description": "**可选**：同父下的目标序号（1 起）。给了就**重排**：同父其他节点自动顺移，序号保持 1..n 连续；换父但没给序号时追加到新父末尾" },
                "clear_storyline": { "type": "boolean", "description": "**可选**：解绑故事线（连 arc_stage 一起清空）" },
                "clear_location": { "type": "boolean", "description": "**可选**：解绑地点" },
                "attributes": { "type": "object", "description": "**可选**：补充属性（自由对象，**整体替换**，不给就不动）。细纲场景的 objective / conflict / pov_character_id / location_id / required_events / forbidden_events 等都放在这里；建节点时漏填的字段用它补齐，**不要**删了重建——场景一旦被伏笔锚点引用，重建会让锚点脱钩" }
            });
            insert_props(&mut props, node_outline_props());
            json!({ "type": "object", "properties": props, "required": ["id"] })
        }
        NarrativeAction::RemoveNode | NarrativeAction::GetNode => json!({
            "type": "object",
            "properties": { "id": { "type": "string" } },
            "required": ["id"]
        }),
        NarrativeAction::ListNodes => json!({
            "type": "object",
            "properties": {
                "limit": { "type": "number", "description": format!("**可选**：本页条数，默认 {}，上限 {}（超过报错）", agent::LIST_DEFAULT_LIMIT, agent::LIST_MAX_LIMIT) },
                "offset": { "type": "number", "description": "**可选**：从第几条开始（默认 0）；返回里的 next_offset 就是下一页该传的值" },
                "parent_id": { "type": "string", "description": "**可选**：只看该父节点下的直接子节点（逐层下钻用）" },
                "roots_only": { "type": "boolean", "description": "**可选**：只看顶层节点（没有父节点的卷 / 弧）。与 parent_id 互斥" },
                "node_type": { "type": "string", "description": format!("**可选**：只看某种节点类型。{}", domain::narrative::NarrativeNodeType::legal_values_hint()) },
                "storyline_id": { "type": "string", "description": "**可选**：只看服务某条故事线的节点" }
            },
            "required": []
        }),
    }
}

#[async_trait]
impl AgentTool for NarrativeTool {
    fn name(&self) -> String {
        narrative_name(self.action).to_string()
    }
    fn description(&self) -> String {
        narrative_description(self.action)
    }
    fn input_schema(&self) -> Value {
        narrative_schema(self.action)
    }

    async fn execute(&self, input: Value) -> Result<Value> {
        match self.action {
            NarrativeAction::CreateNode => {
                let project_id = parse_uuid(&input, "project_id")?;
                let node_type = opt_str(&input, "node_type")
                    .ok_or_else(|| anyhow::anyhow!("node_type 缺失"))?
                    .to_string();
                let title = opt_str(&input, "title")
                    .ok_or_else(|| anyhow::anyhow!("title 缺失"))?
                    .to_string();
                let outline = node_outline_patch_from(&input)?;
                let new_node = domain::narrative::NewNarrativeNode {
                    project_id,
                    node_type,
                    parent_id: outline.parent_id,
                    title,
                    description: opt_str_owned(&input, "description"),
                    content: opt_str_owned(&input, "content"),
                    attributes: attributes_arg(&input)?,
                    sort_order: outline.sort_order,
                    status: opt_str_owned(&input, "status"),
                    storyline_id: outline.storyline_id,
                    arc_stage: outline.arc_stage,
                    stage_refs: outline.stage_refs.unwrap_or_default(),
                    participant_entity_ids: outline.participant_entity_ids.unwrap_or_default(),
                    location_id: outline.location_id,
                    item_ids: outline.item_ids.unwrap_or_default(),
                    estimated_chapters: outline.estimated_chapters,
                    estimated_words: outline.estimated_words,
                    story_time: outline.story_time,
                };
                let node = self.service.create_node(new_node).await?;
                Ok(json!({ "ok": true, "action": "create_node", "data": node }))
            }
            NarrativeAction::ReviseNode => {
                let id = parse_uuid(&input, "id")?;
                // append=true：description / content 追加写入。
                // 章节正文是系统里最长的文本，被输出上限截断时靠它分次续写。
                let append = opt_bool(&input, "append")?;
                let mut description: Option<String> = opt_str(&input, "description").map(str::to_string);
                let mut content: Option<String> = opt_str(&input, "content").map(str::to_string);
                if append {
                    let existing = self
                        .service
                        .get_node(id)
                        .await?
                        .ok_or_else(|| anyhow::anyhow!("叙事节点不存在: {}", id))?;
                    let old_description = existing
                        .get("description")
                        .and_then(|v| v.as_str())
                        .map(str::to_string);
                    let old_content = existing
                        .get("content")
                        .and_then(|v| v.as_str())
                        .map(str::to_string);
                    description = description
                        .map(|s| agent::append_or_replace(old_description.as_deref(), &s, true));
                    content = content
                        .map(|s| agent::append_or_replace(old_content.as_deref(), &s, true));
                }
                let outline = node_outline_patch_from(&input)?;
                let node = self
                    .service
                    .update_node(
                        id,
                        opt_str(&input, "title"),
                        description.as_deref(),
                        content.as_deref(),
                        opt_str(&input, "status"),
                        opt_attributes_arg(&input)?,
                        outline,
                    )
                    .await?;
                Ok(json!({ "ok": true, "action": "revise_node", "data": node }))
            }
            NarrativeAction::RemoveNode => {
                let id = parse_uuid(&input, "id")?;
                self.service.delete_node(id).await?;
                Ok(json!({ "ok": true, "action": "retire_node", "id": id.to_string() }))
            }
            NarrativeAction::GetNode => {
                let id = parse_uuid(&input, "id")?;
                let node = self
                    .service
                    .get_node(id)
                    .await?
                    .ok_or_else(|| anyhow::anyhow!("叙事节点不存在: {}", id))?;
                Ok(json!({ "ok": true, "action": "get_node", "data": node }))
            }
            NarrativeAction::ListNodes => {
                let project_id = parse_uuid(&input, "project_id")?;
                let (limit, offset) = agent::parse_page_args(&input)?;
                let filter = domain::narrative::NarrativeNodeFilter {
                    parent_id: opt_uuid_strict(&input, "parent_id")?,
                    roots_only: opt_bool(&input, "roots_only")?,
                    node_type: opt_str_owned(&input, "node_type"),
                    storyline_id: opt_uuid_strict(&input, "storyline_id")?,
                };
                let filtered = filter.parent_id.is_some()
                    || filter.roots_only
                    || filter.node_type.is_some()
                    || filter.storyline_id.is_some();
                let (items, total) = self
                    .service
                    .list_nodes_page(project_id, &filter, limit, offset)
                    .await?;
                // 目录页投影：只给列表需要的字段（实测不投影时这些列表可到上万字符）。
                // child_count 让模型不必为了「这一层还有几条」再拉一次全量。
                let data: Vec<Value> = items
                    .iter()
                    .map(|x| {
                        agent::pick_fields(
                            x,
                            &[
                                "id",
                                "title",
                                "node_type",
                                "status",
                                "parent_id",
                                "sort_order",
                                "child_count",
                                "storyline_id",
                                "storyline_name",
                                "arc_stage",
                            ],
                        )
                    })
                    .collect();
                // 过滤生效时在信封里显式回带条件：否则模型看到一个较小的 total，
                // 很容易误以为「项目里就这么多节点」
                let extras: Vec<(&str, Value)> = if filtered {
                    vec![(
                        "filter",
                        json!({
                            "parent_id": filter.parent_id,
                            "roots_only": filter.roots_only,
                            "node_type": filter.node_type,
                            "storyline_id": filter.storyline_id,
                            "note": "total 是**命中过滤**的条数，不是项目全量",
                        }),
                    )]
                } else {
                    vec![]
                };
                Ok(agent::list_envelope(
                    "list_nodes", data, total, limit, offset, "叙事节点", extras,
                ))
            }
        }
    }
}

/// 单次 `bulk_create_nodes` 的条目上限。
///
/// 与人物 / 地点档案的批量工具同一考虑：一次几百项会同时撞上 JSON 长度与超时，
/// 宁可让模型自己分批（建一棵长篇小说细纲通常 20~40 个节点一批正合适）。
pub const BULK_CREATE_NODES_MAX_ITEMS: usize = 50;

/// 批量新建叙事节点（细纲建树）。
///
/// 逐条执行、逐条报错（不静默跳过）：失败的条目进 `failures` 并带上原因，
/// 成功的条目进 `created` 并回带新 id。
///
/// 树形引用：同一批次内可以用 `key` 给节点起临时标识，后面的条目用
/// `parent_key` 引用它——一次调用就能建出「卷 → 弧 → 章」整棵子树。
pub struct BulkCreateNodesTool {
    service: Arc<NarrativeService>,
}

impl BulkCreateNodesTool {
    pub fn new(service: Arc<NarrativeService>) -> Self {
        Self { service }
    }
}

/// 批量建节点的条目 schema = 单条 create_node 的 schema + `key` / `parent_key`。
fn bulk_create_nodes_item_schema() -> Value {
    let mut schema = narrative_schema(NarrativeAction::CreateNode);
    if let Some(props) = schema.get_mut("properties") {
        insert_props(
            props,
            vec![
                (
                    "key",
                    json!({ "type": "string", "description": "**可选**：本批次内的临时标识（例如 vol1）。同一批次里后面的节点可以用 parent_key 引用它——一次调用建出整棵子树" }),
                ),
                (
                    "parent_key",
                    json!({ "type": "string", "description": "**可选**：父节点用同批次里更早出现的 key 引用。与 parent_id 互斥" }),
                ),
            ],
        );
    }
    schema
}

#[async_trait]
impl AgentTool for BulkCreateNodesTool {
    fn name(&self) -> String {
        "bulk_create_nodes".to_string()
    }

    fn description(&self) -> String {
        format!(
            "一次按顺序新建多个叙事节点（最多 {} 项），用于把细纲整棵树建出来。\
             同一批次里可以用 key 给节点起临时标识，后面的条目用 parent_key 引用它，\
             因此「先建卷、再在同一批里建它下面的弧和章」不需要分多轮。\
             逐条执行、逐条报错：失败的条目会带上原因返回，不会静默跳过。\
             每条字段与 create_node 完全一致（project_id 由框架注入，不要手写）。",
            BULK_CREATE_NODES_MAX_ITEMS
        )
    }

    /// 聚合器：一次代替多次调用，预算按累计口径给（与 batch_call 同理）。
    fn result_budget(&self) -> usize {
        agent::BATCH_RESULT_MAX_CHARS
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "nodes": {
                    "type": "array",
                    "description": format!("要新建的节点数组，按父在前、子在后排列（parent_key 只能引用更早出现的 key）。建议每批 20~40 条，上限 {} 条", BULK_CREATE_NODES_MAX_ITEMS),
                    "items": bulk_create_nodes_item_schema()
                }
            },
            "required": ["nodes"]
        })
    }

    async fn execute(&self, input: Value) -> Result<Value> {
        let project_id = parse_uuid(&input, "project_id")?;
        let arr = input
            .get("nodes")
            .and_then(|v| v.as_array())
            .ok_or_else(|| anyhow::anyhow!("nodes 必须是非空数组"))?;
        if arr.is_empty() {
            anyhow::bail!("nodes 不能为空：至少要给一个节点");
        }
        if arr.len() > BULK_CREATE_NODES_MAX_ITEMS {
            anyhow::bail!(
                "nodes 有 {} 项，超过单次上限 {} 项：请拆成多次 bulk_create_nodes",
                arr.len(),
                BULK_CREATE_NODES_MAX_ITEMS
            );
        }

        let creator = NarrativeTool::new(NarrativeAction::CreateNode, self.service.clone());
        // 本批次内 key -> 新节点 id
        let mut key_map: BTreeMap<String, Uuid> = BTreeMap::new();
        let mut created: Vec<Value> = Vec::with_capacity(arr.len());
        let mut failures: Vec<Value> = Vec::new();

        for (index, item) in arr.iter().enumerate() {
            let mut payload = item.clone();
            let Some(obj) = payload.as_object_mut() else {
                failures.push(json!({ "index": index, "error": "条目应为对象" }));
                continue;
            };
            // key / parent_key 是本工具的编排字段，不能透传给 create_node
            // （否则会被「字段名不存在」拦下，或者更糟：被当成未知字段忽略）
            let key = obj
                .remove("key")
                .and_then(|v| v.as_str().map(str::trim).filter(|s| !s.is_empty()).map(str::to_string));
            let parent_key = obj
                .remove("parent_key")
                .and_then(|v| v.as_str().map(str::trim).filter(|s| !s.is_empty()).map(str::to_string));

            if let Some(pk) = parent_key.as_deref() {
                if obj.contains_key("parent_id") {
                    failures.push(json!({
                        "index": index,
                        "key": key,
                        "error": "parent_key 与 parent_id 不能同时给",
                    }));
                    continue;
                }
                match key_map.get(pk) {
                    Some(pid) => {
                        obj.insert("parent_id".to_string(), json!(pid.to_string()));
                    }
                    None => {
                        failures.push(json!({
                            "index": index,
                            "key": key,
                            "error": format!("parent_key「{}」在本批次里找不到：它必须引用**同一批次里更早出现**的 key", pk),
                        }));
                        continue;
                    }
                }
            }

            obj.insert("project_id".to_string(), json!(project_id.to_string()));
            match creator.execute(payload).await {
                Ok(value) => {
                    let data = value.get("data").cloned().unwrap_or_else(|| json!({}));
                    let id = data.get("id").and_then(|v| v.as_str()).map(str::to_string);
                    if let (Some(k), Some(id_str)) = (key.clone(), id.clone()) {
                        match Uuid::parse_str(&id_str) {
                            Ok(uuid) => {
                                if key_map.insert(k.clone(), uuid).is_some() {
                                    failures.push(json!({
                                        "index": index,
                                        "key": k,
                                        "error": "key 在本批次里重复：节点本身已创建，但后续 parent_key 引用它会有歧义",
                                    }));
                                }
                            }
                            Err(_) => failures.push(json!({
                                "index": index,
                                "key": k,
                                "error": format!("新建节点返回的 id 不是合法 UUID：{}", id_str),
                            })),
                        }
                    }
                    created.push(json!({
                        "index": index,
                        "key": key,
                        "id": id,
                        "title": data.get("title"),
                        "node_type": data.get("node_type"),
                        "parent_id": data.get("parent_id"),
                        "sort_order": data.get("sort_order"),
                    }));
                }
                Err(e) => failures.push(json!({
                    "index": index,
                    "key": key,
                    "title": item.get("title"),
                    "error": e.to_string(),
                })),
            }
        }

        Ok(json!({
            "ok": failures.is_empty(),
            "action": "bulk_create_nodes",
            "created_count": created.len(),
            "created": created,
            "failures": failures,
        }))
    }
}

// ============================================================
// Storyline 聚合
// ============================================================

#[derive(Clone, Copy)]
pub enum StorylineAction {
    CreateStoryline,
    ReviseStoryline,
    RetireStoryline,
    /// 按 id 读单条：比 list_storylines 省上下文（列表会返回所有线的全文）
    GetStoryline,
    ListStorylines,
    /// 建一条带类型的剧情线关系（驱动 / 依赖 / 交汇 / 对冲 / 包含）
    RelateStorylines,
    /// 删除一条剧情线关系
    UnrelateStorylines,
    /// 列出剧情线之间的关系边（含类型与两端名字）
    ListStorylineRelations,
}

pub struct StorylineTool {
    action: StorylineAction,
    service: Arc<StorylineService>,
}

impl StorylineTool {
    pub fn new(action: StorylineAction, service: Arc<StorylineService>) -> Self {
        Self { action, service }
    }
}

pub fn register_storyline_tools(registry: &ToolRegistry, service: Arc<StorylineService>) {
    for a in [
        StorylineAction::CreateStoryline,
        StorylineAction::ReviseStoryline,
        StorylineAction::RetireStoryline,
        StorylineAction::GetStoryline,
        StorylineAction::ListStorylines,
        StorylineAction::RelateStorylines,
        StorylineAction::UnrelateStorylines,
        StorylineAction::ListStorylineRelations,
    ] {
        registry.register(Arc::new(StorylineTool::new(a, service.clone())));
    }
}

fn storyline_name(a: StorylineAction) -> &'static str {
    match a {
        StorylineAction::CreateStoryline => "create_storyline",
        StorylineAction::ReviseStoryline => "revise_storyline",
        StorylineAction::RetireStoryline => "retire_storyline",
        StorylineAction::GetStoryline => "get_storyline",
        StorylineAction::RelateStorylines => "relate_storylines",
        StorylineAction::UnrelateStorylines => "unrelate_storylines",
        StorylineAction::ListStorylineRelations => "list_storyline_relations",
        StorylineAction::ListStorylines => "list_storylines",
    }
}

fn storyline_description(a: StorylineAction) -> String {
    match a {
        StorylineAction::CreateStoryline => "创建跨卷剧情线（默认 status=Planned）。\n\
            importance='Main' = 主线（每项目 1 条）；\n\
            importance='Normal'/'Important'/'Minor' = 副线；\n\
            tone='light' = 明线（用户可见）；tone='dark' = 暗线（伏笔/钩子）；\n\
            visibility='hidden' 一般配合 tone='dark' 用；\n\
            parent_id=挂到某条 storyline 下，副线必须挂到主线或其他副线。".into(),
        StorylineAction::ReviseStoryline => "修改剧情线名称 / 描述。这会修改已有产物。".into(),
        StorylineAction::RetireStoryline => "语义化结束一条剧情线（**即 delete_storyline**，逻辑删除）。删除前请先用 list_storylines 确认目标 id。".into(),
        StorylineAction::GetStoryline => {
            "按 id 读取**单条**剧情线（含 arc_stages 阶段弧线）。\
             已经知道 id 时用它，不要用 list_storylines 拉全部——列表会把每条线的全文都返回，\
             白占上下文（本项目 storyline 全文近万字）。"
                .into()
        }
        StorylineAction::RelateStorylines => {
            "给两条剧情线建一条**带类型的关系边**（同向已存在则更新类型，幂等）。\
             类型：Contains 包含 / Drives 驱动 / DependsOn 依赖 / Intersects 交汇 / Counters 对冲\
             （也可写中文）。用它表达「A 推进得越多，B 压力越大」这类横向咬合——\
             只靠 create_storyline 的 parent_id 只能挂成一棵树，画不出线咬合图。"
                .into()
        }
        StorylineAction::UnrelateStorylines => {
            "删除一条剧情线关系边（按关系 id，用 list_storyline_relations 查）。\
             只删边，不动任何剧情线本身。"
                .into()
        }
        StorylineAction::ListStorylineRelations => {
            "列出剧情线之间的**关系边**（含类型 relation_type 与两端名字 from_name / to_name）。\
             回答「这几条线怎么咬合 / 谁在驱动谁」时用它，不要逐条读剧情线正文。"
                .into()
        }
        StorylineAction::ListStorylines => "列出某项目的全部剧情线，用于检索上下文。".into(),
    }
}

fn storyline_schema(a: StorylineAction) -> Value {
    match a {
        StorylineAction::CreateStoryline => {
            let mut props = json!({
                "name": { "type": "string" },
                "description": { "type": "string" },
                "status": {
                    "type": "string",
                    "description": format!("可选，默认 Planned。取值：{}（也可写中文：计划中 / 进行中 / 已解决 / 已放弃）", sl_domain::STORYLINE_STATUSES.join(" / "))
                },
                "importance": { "type": "string", "description": "可选：Main(主线) / Important / Normal / Minor" },
                "tone": { "type": "string", "description": "明/暗线：light(明) / dark(暗)" },
                "visibility": { "type": "string", "description": "可见性：visible / hidden（暗线一般 hidden）" },
                "parent_id": { "type": "string", "description": "挂载到哪条 storyline 下（None 表示独立）" }
            });
            // 阶段弧线：与人物 / 势力 / 地点**同构**（同一套字段与归一化）
            splice_arc_stage_props(&mut props);
            json!({ "type": "object", "properties": props, "required": ["name"] })
        },
        StorylineAction::ReviseStoryline => {
            let mut props = json!({
                "id": { "type": "string" },
                "name": { "type": "string" },
                "description": { "type": "string" },
                "status": {
                    "type": "string",
                    "description": format!("推进剧情线状态（不传则保持原值）。取值：{}", sl_domain::STORYLINE_STATUSES.join(" / "))
                },
                "tone": { "type": "string", "description": "明/暗线：light(明) / dark(暗)（不传则保持原值）" },
                "visibility": { "type": "string", "description": "可见性：visible / hidden（不传则保持原值）" }
            });
            // 阶段弧线：与人物 / 势力 / 地点同构（含 arc_stages_mode / remove_arc_stages）
            splice_arc_stage_props(&mut props);
            json!({ "type": "object", "properties": props, "required": ["id"] })
        },
        StorylineAction::RetireStoryline => json!({
            "type": "object",
            "properties": { "id": { "type": "string" } },
            "required": ["id"]
        }),
        StorylineAction::RelateStorylines => json!({
            "type": "object",
            "properties": {
                "from_storyline_id": { "type": "string", "description": "起点剧情线 UUID（用 list_storylines 取得）" },
                "to_storyline_id": { "type": "string", "description": "终点剧情线 UUID（方向有意义：驱动 / 依赖都是单向的）" },
                "relation_type": {
                    "type": "string",
                    "description": format!("可选，默认 Contains。取值：{}（也可写中文：包含 / 驱动 / 依赖 / 交汇 / 对冲）", sl_domain::STORYLINE_RELATION_TYPES.join(" / "))
                }
            },
            "required": ["from_storyline_id", "to_storyline_id"]
        }),
        StorylineAction::UnrelateStorylines => json!({
            "type": "object",
            "properties": { "id": { "type": "string", "description": "关系 id（用 list_storyline_relations 查）" } },
            "required": ["id"]
        }),
        StorylineAction::ListStorylineRelations => json!({
            "type": "object",
            "properties": {
                "limit": { "type": "number", "description": format!("**可选**：本页条数，默认 {}，上限 {}（超过报错）", agent::LIST_DEFAULT_LIMIT, agent::LIST_MAX_LIMIT) },
                "offset": { "type": "number", "description": "**可选**：从第几条开始（默认 0）" }
            },
            "required": []
        }),
        StorylineAction::GetStoryline => json!({
            "type": "object",
            "properties": { "id": { "type": "string", "description": "剧情线 UUID（用 list_storylines 取得）" } },
            "required": ["id"]
        }),
        StorylineAction::ListStorylines => json!({
            "type": "object",
            "properties": {
                "limit": { "type": "number", "description": format!("**可选**：本页条数，默认 {}，上限 {}（超过报错）", agent::LIST_DEFAULT_LIMIT, agent::LIST_MAX_LIMIT) },
                "offset": { "type": "number", "description": "**可选**：从第几条开始（默认 0）；返回里的 next_offset 就是下一页该传的值" }
            },
            "required": []
        }),
    }
}

#[async_trait]
impl AgentTool for StorylineTool {
    fn name(&self) -> String {
        storyline_name(self.action).to_string()
    }
    fn description(&self) -> String {
        storyline_description(self.action)
    }
    fn input_schema(&self) -> Value {
        storyline_schema(self.action)
    }

    async fn execute(&self, input: Value) -> Result<Value> {
        match self.action {
            StorylineAction::CreateStoryline => {
                let project_id = parse_uuid(&input, "project_id")?;
                let name = opt_str(&input, "name").ok_or_else(|| anyhow::anyhow!("name 缺失"))?;
                let parent_uuid = match input.get("parent_id").and_then(|v| v.as_str()) {
                    Some(s) if !s.is_empty() => Some(Uuid::parse_str(s).map_err(|e| anyhow::anyhow!("parent_id 非法: {}", e))?),
                    _ => None,
                };
                // 枚举入参一律经 domain 枚举校验：非法值在这里报错，
                // 不再把脏字符串写进库（写进去后前端按状态筛选就全失准了）。
                let status = match opt_enum_arg(
                    &input,
                    "status",
                    |s| sl_domain::StorylineStatus::parse(s).map(|v| v.as_str()),
                    sl_domain::STORYLINE_STATUSES,
                )? {
                    Some(v) => v,
                    None => "Planned",
                };
                let importance = match opt_enum_arg(
                    &input,
                    "importance",
                    |s| sl_domain::StorylineImportance::parse(s).map(|v| v.as_str()),
                    sl_domain::STORYLINE_IMPORTANCES,
                )? {
                    Some(v) => v,
                    None => "Normal",
                };
                let tone = match opt_enum_arg(
                    &input,
                    "tone",
                    |s| sl_domain::StorylineTone::parse(s).map(|v| v.as_str()),
                    &["light", "dark"],
                )? {
                    Some(v) => v,
                    None => "light",
                };
                let visibility = match opt_enum_arg(
                    &input,
                    "visibility",
                    |s| sl_domain::StorylineVisibility::parse(s).map(|v| v.as_str()),
                    &["visible", "hidden"],
                )? {
                    Some(v) => v,
                    None => "visible",
                };

                // 阶段弧线（可选）：与人物 / 势力 / 地点同一套归一化
                // （字符串简写 → {stage}、screen_weight 中文规范化、role/stage_role 等价键）
                let arc_stages = match input.get("arc_stages") {
                    Some(v) if !v.is_null() => {
                        let arr = v
                            .as_array()
                            .ok_or_else(|| anyhow::anyhow!("arc_stages 应为数组"))?;
                        let normalized: Vec<Value> = arr
                            .iter()
                            .map(normalize_arc_stage_item)
                            .collect::<Result<Vec<_>>>()?;
                        Some(Value::Array(normalized))
                    }
                    _ => None,
                };
                let s = self
                    .service
                    .create_storyline(
                        project_id,
                        name,
                        opt_str(&input, "description"),
                        status,
                        importance,
                        tone,
                        visibility,
                        parent_uuid,
                        arc_stages.as_ref(),
                    )
                    .await?;
                Ok(json!({ "ok": true, "action": "create_storyline", "data": s }))
            }
            StorylineAction::ReviseStoryline => {
                let id = parse_uuid(&input, "id")?;
                let status = opt_enum_arg(
                    &input,
                    "status",
                    |s| sl_domain::StorylineStatus::parse(s).map(|v| v.as_str()),
                    sl_domain::STORYLINE_STATUSES,
                )?;
                let name = opt_str(&input, "name");
                let description = opt_str(&input, "description");
                let tone = opt_str(&input, "tone");
                let visibility = opt_str(&input, "visibility");

                // 阶段弧线：与人物 / 势力 / 地点**同一套 merge 语义**
                // （默认整块替换；arc_stages_mode=merge 时按 stage 名合并、未提及的保留；
                //  只传 remove_arc_stages 时按阶段名删）。合并要先拿旧值，所以读一次。
                let remove_arc_stages = string_array(input.get("remove_arc_stages"));
                let obj = input
                    .as_object()
                    .ok_or_else(|| anyhow::anyhow!("入参应为 JSON 对象"))?;
                let arc_stages = match input.get("arc_stages") {
                    Some(v) if !v.is_null() => {
                        let existing = self
                            .service
                            .get_storyline(id)
                            .await?
                            .ok_or_else(|| anyhow::anyhow!("剧情线不存在: {}", id))?;
                        let mode = arc_stages_mode_of(obj, &remove_arc_stages);
                        Some(apply_arc_stages(
                            existing.get("arc_stages"),
                            v,
                            &mode,
                            &remove_arc_stages,
                        )?)
                    }
                    _ if !remove_arc_stages.is_empty() => {
                        let existing = self
                            .service
                            .get_storyline(id)
                            .await?
                            .ok_or_else(|| anyhow::anyhow!("剧情线不存在: {}", id))?;
                        Some(remove_arc_stages_only(
                            existing.get("arc_stages"),
                            &remove_arc_stages,
                        ))
                    }
                    _ => None,
                };

                // 一个要改的字段都没给：报错，而不是"成功但什么都没变"
                if name.is_none()
                    && description.is_none()
                    && status.is_none()
                    && tone.is_none()
                    && visibility.is_none()
                    && arc_stages.is_none()
                {
                    anyhow::bail!(
                        "revise_storyline 至少要给一个要改的字段：name / description / status / tone / visibility / arc_stages"
                    );
                }
                let s = self
                    .service
                    .update_storyline(
                        id,
                        name.as_deref(),
                        description.as_deref(),
                        status,
                        tone.as_deref(),
                        visibility.as_deref(),
                        arc_stages.as_ref(),
                    )
                    .await?;
                Ok(json!({ "ok": true, "action": "revise_storyline", "data": s }))
            }
            StorylineAction::RetireStoryline => {
                let id = parse_uuid(&input, "id")?;
                self.service.delete_storyline(id).await?;
                Ok(json!({ "ok": true, "action": "retire_storyline", "id": id.to_string() }))
            }
            StorylineAction::RelateStorylines => {
                let project_id = parse_uuid(&input, "project_id")?;
                let from = parse_uuid(&input, "from_storyline_id")?;
                let to = parse_uuid(&input, "to_storyline_id")?;
                let relation_type = opt_enum_arg(
                    &input,
                    "relation_type",
                    |s| sl_domain::StorylineRelationType::parse(s).map(|v| v.as_str()),
                    sl_domain::STORYLINE_RELATION_TYPES,
                )?
                .unwrap_or("Contains");
                let r = self
                    .service
                    .relate_storylines(project_id, from, to, relation_type)
                    .await?;
                Ok(json!({ "ok": true, "action": "relate_storylines", "data": r }))
            }
            StorylineAction::UnrelateStorylines => {
                let id = parse_uuid(&input, "id")?;
                self.service.unrelate_storylines(id).await?;
                Ok(json!({ "ok": true, "action": "unrelate_storylines", "id": id.to_string() }))
            }
            StorylineAction::ListStorylineRelations => {
                let project_id = parse_uuid(&input, "project_id")?;
                let all = self.service.list_storyline_relations(project_id).await?;
                // 附上两端名字：只给 uuid 的话调用方还得再查一次才知道是哪两条线
                let storylines = self.service.list_storylines(project_id).await?;
                let mut names: std::collections::HashMap<String, String> =
                    std::collections::HashMap::new();
                for s in &storylines {
                    if let (Some(id), Some(name)) = (
                        s.get("id").and_then(|v| v.as_str()),
                        s.get("name").and_then(|v| v.as_str()),
                    ) {
                        names.insert(id.to_string(), name.to_string());
                    }
                }
                let (limit, offset) = agent::parse_page_args(&input)?;
                let total = all.len();
                let data: Vec<Value> = all
                    .iter()
                    .skip(offset)
                    .take(limit)
                    .map(|r| {
                        let mut item = r.clone();
                        for (id_key, name_key) in
                            [("parent_id", "from_name"), ("child_id", "to_name")]
                        {
                            if let Some(id) = item.get(id_key).and_then(|v| v.as_str()) {
                                if let Some(n) = names.get(id) {
                                    item[name_key] = json!(n);
                                }
                            }
                        }
                        item
                    })
                    .collect();
                Ok(agent::list_envelope(
                    "list_storyline_relations",
                    data,
                    total,
                    limit,
                    offset,
                    "剧情线关系",
                    vec![],
                ))
            }
            StorylineAction::GetStoryline => {
                let id = parse_uuid(&input, "id")?;
                let s = self
                    .service
                    .get_storyline(id)
                    .await?
                    .ok_or_else(|| anyhow::anyhow!("剧情线不存在: {}", id))?;
                Ok(json!({ "ok": true, "action": "get_storyline", "data": s }))
            }
            StorylineAction::ListStorylines => {
                let project_id = parse_uuid(&input, "project_id")?;
                let (limit, offset) = agent::parse_page_args(&input)?;
                let (items, total) = self.service.list_storylines_page(project_id, limit, offset).await?;
                // 目录页投影：只给列表需要的字段（实测不投影时这些列表可到上万字符）
                let data: Vec<Value> = items
                    .iter()
                    .map(|x| agent::pick_fields(x, &["id", "name", "description", "importance", "status", "tone", "visibility"]))
                    .collect();
                Ok(agent::list_envelope("list_storylines", data, total, limit, offset, "剧情线", vec![]))
            }
        }
    }
}

// ============================================================
// Foreshadow 聚合
// ============================================================

#[derive(Clone, Copy)]
pub enum ForeshadowAction {
    CreateForeshadow,
    ReviseForeshadow,
    RetireForeshadow,
    ListForeshadows,
}

pub struct ForeshadowTool {
    action: ForeshadowAction,
    service: Arc<ForeshadowService>,
    /// 用来校验"埋点 / 回收点 / 父伏笔"确实存在（表上没有外键，见 ensure_* 注释）。
    pool: PgPool,
}

impl ForeshadowTool {
    pub fn new(action: ForeshadowAction, service: Arc<ForeshadowService>, pool: PgPool) -> Self {
        Self {
            action,
            service,
            pool,
        }
    }

    /// 校验伏笔锚点指向的目标确实存在：**不写悬空 uuid**。
    ///
    /// 本表历来没有外键（跨表一致性由应用层保证），所以校验放在写入前：
    /// 节点 / 父伏笔不存在就直接报错，而不是在库里留一个查不出名字的 id。
    async fn ensure_node_exists(&self, id: Uuid) -> Result<()> {
        let exists: Option<(i32,)> =
            sqlx::query_as("SELECT 1 FROM narrative_node WHERE id = $1")
                .bind(id)
                .fetch_optional(&self.pool)
                .await
                .context("校验叙事节点是否存在失败")?;
        if exists.is_none() {
            anyhow::bail!(
                "叙事节点 {} 不存在：埋点 / 回收点必须指向真实节点（先用 list_nodes 查）",
                id
            );
        }
        Ok(())
    }

    async fn ensure_foreshadow_exists(&self, id: Uuid) -> Result<()> {
        let exists: Option<(i32,)> =
            sqlx::query_as("SELECT 1 FROM foreshadowing WHERE id = $1")
                .bind(id)
                .fetch_optional(&self.pool)
                .await
                .context("校验父伏笔是否存在失败")?;
        if exists.is_none() {
            anyhow::bail!("父伏笔 {} 不存在：parent_foreshadow_id 必须指向真实伏笔", id);
        }
        Ok(())
    }

    /// 解析并校验三个锚点（不存在即报错）。
    async fn resolve_anchors(
        &self,
        input: &Value,
    ) -> Result<(Option<Uuid>, Option<Uuid>, Option<Uuid>)> {
        let planted = opt_uuid_strict(input, "planted_node_id")?;
        let payoff = opt_uuid_strict(input, "payoff_node_id")?;
        let parent = opt_uuid_strict(input, "parent_foreshadow_id")?;
        if let Some(n) = planted {
            self.ensure_node_exists(n).await?;
        }
        if let Some(n) = payoff {
            self.ensure_node_exists(n).await?;
        }
        if let Some(f) = parent {
            self.ensure_foreshadow_exists(f).await?;
        }
        Ok((planted, payoff, parent))
    }
}

pub fn register_foreshadow_tools(
    registry: &ToolRegistry,
    service: Arc<ForeshadowService>,
    pool: PgPool,
) {
    for a in [
        ForeshadowAction::CreateForeshadow,
        ForeshadowAction::ReviseForeshadow,
        ForeshadowAction::RetireForeshadow,
        ForeshadowAction::ListForeshadows,
    ] {
        registry.register(Arc::new(ForeshadowTool::new(
            a,
            service.clone(),
            pool.clone(),
        )));
    }
}

fn foreshadow_name(a: ForeshadowAction) -> &'static str {
    match a {
        ForeshadowAction::CreateForeshadow => "create_foreshadow",
        ForeshadowAction::ReviseForeshadow => "revise_foreshadow",
        ForeshadowAction::RetireForeshadow => "retire_foreshadow",
        ForeshadowAction::ListForeshadows => "list_foreshadows",
    }
}

fn foreshadow_description(a: ForeshadowAction) -> String {
    match a {
        ForeshadowAction::CreateForeshadow => {
            "创建伏笔（含重要度与提示等级）。**建议同时传 storyline_id** 把它挂到所属剧情线上——             未挂线的伏笔是孤儿：前端无法「点开一条暗线看它埋了哪些钩子」，             长篇连载里这是最有用的视图之一。先用 list_storylines 取剧情线 id。"
                .into()
        }
        ForeshadowAction::ReviseForeshadow => {
            "修改伏笔名称 / 描述 / 状态，并可改挂或解除剧情线归属（storyline_id / clear_storyline）。             **只传要改的字段，其余保持原值**——只想挂线时不必把名字再抄一遍。这会修改已有产物。"
                .into()
        }
        ForeshadowAction::RetireForeshadow => "语义化结束一条伏笔（**即 delete_foreshadow**，逻辑删除）。删除前请先用 list_foreshadows 确认目标 id。".into(),
        ForeshadowAction::ListForeshadows => {
            "列出某项目的全部伏笔（含所属剧情线 storyline_id / storyline_name；             二者为 null 即「孤儿伏笔」，需要决定挂到哪条线上），用于检索上下文。"
                .into()
        }
    }
}

fn foreshadow_schema(a: ForeshadowAction) -> Value {
    match a {
        ForeshadowAction::CreateForeshadow => json!({
            "type": "object",
            "properties": {
                "name": { "type": "string" },
                "description": { "type": "string" },
                "status": {
                    "type": "string",
                    "description": format!("可选，默认 Planned。取值：{}（也可写中文：计划中 / 已引入 / 进行中 / 已揭示 / 已放弃）", fs_domain::FORESHADOWING_STATUSES.join(" / "))
                },
                "importance": {
                    "type": "string",
                    "description": format!("可选，默认 Normal。取值：{}（也可写中文：核心 / 重要 / 普通 / 次要）", fs_domain::FORESHADOWING_IMPORTANCES.join(" / "))
                },
                "hint_level": {
                    "type": "string",
                    "description": format!("可选，默认 Subtle。取值：{}（也可写中文：明示 / 直接 / 隐晦 / 隐藏）", fs_domain::HINT_LEVELS.join(" / "))
                },
                "hint_note": { "type": "string", "description": "可选：等级之外的**备注**（如「前期只透风，不揭示」）。等级是枚举、说明是自由文本，两者分开写——原先混在 hint_level 里，前端没法按等级筛选" },
                "introduced_at": { "type": "string", "description": "可选：**埋点时机**（自由文本，如「前期」「第三卷」「第 12 章」）" },
                "expected_reveal_at": { "type": "string", "description": "可选：**预期回收时机**（自由文本）。连载里「这条钩子埋在哪、打算哪章收」是刚需" },
                "planted_node_id": { "type": "string", "description": "可选：**埋点所在节点**（哪一章埋的，用 list_nodes 取 id）。必须指向真实节点" },
                "payoff_node_id": { "type": "string", "description": "可选：**计划回收所在节点**（打算哪一章收）" },
                "parent_foreshadow_id": { "type": "string", "description": "可选：父伏笔 id —— 一条大伏笔挂几个小钩子（伏笔树）" },
                "storyline_id": { "type": "string", "description": "所属剧情线 UUID（建议填：不填就是孤儿伏笔）。用 list_storylines 取得" }
            },
            "required": ["name"]
        }),
        ForeshadowAction::ReviseForeshadow => json!({
            "type": "object",
            "properties": {
                "id": { "type": "string" },
                "name": { "type": "string" },
                "description": { "type": "string" },
                "status": {
                    "type": "string",
                    "description": format!("推进伏笔状态（不传则保持原值）。取值：{}", fs_domain::FORESHADOWING_STATUSES.join(" / "))
                },
                "hint_note": { "type": "string", "description": "等级之外的备注（不传则保持原值）" },
                "introduced_at": { "type": "string", "description": "埋点时机（不传则保持原值）" },
                "expected_reveal_at": { "type": "string", "description": "预期回收时机（不传则保持原值）" },
                "actual_reveal_at": { "type": "string", "description": "**实际回收时机**：真收线时填上（不传则保持原值）。与 expected 对照就能看出哪些钩子拖了" },
                "planted_node_id": { "type": "string", "description": "埋点所在节点（不传则保持原值）" },
                "payoff_node_id": { "type": "string", "description": "计划回收所在节点（不传则保持原值）" },
                "parent_foreshadow_id": { "type": "string", "description": "父伏笔 id（不传则保持原值）" },
                "storyline_id": { "type": "string", "description": "改挂到该剧情线（不传则保持原归属）" },
                "clear_storyline": { "type": "boolean", "description": "设为 true 解除剧情线归属（变回孤儿伏笔）" }
            },
            "required": ["id"]
        }),
        ForeshadowAction::RetireForeshadow => json!({
            "type": "object",
            "properties": { "id": { "type": "string" } },
            "required": ["id"]
        }),
        ForeshadowAction::ListForeshadows => json!({
            "type": "object",
            "properties": {
                "limit": { "type": "number", "description": format!("**可选**：本页条数，默认 {}，上限 {}（超过报错）", agent::LIST_DEFAULT_LIMIT, agent::LIST_MAX_LIMIT) },
                "offset": { "type": "number", "description": "**可选**：从第几条开始（默认 0）；返回里的 next_offset 就是下一页该传的值" }
            },
            "required": []
        }),
    }
}

#[async_trait]
impl AgentTool for ForeshadowTool {
    fn name(&self) -> String {
        foreshadow_name(self.action).to_string()
    }
    fn description(&self) -> String {
        foreshadow_description(self.action)
    }
    fn input_schema(&self) -> Value {
        foreshadow_schema(self.action)
    }

    async fn execute(&self, input: Value) -> Result<Value> {
        match self.action {
            ForeshadowAction::CreateForeshadow => {
                let project_id = parse_uuid(&input, "project_id")?;
                let name = opt_str(&input, "name").ok_or_else(|| anyhow::anyhow!("name 缺失"))?;
                let storyline_id = opt_uuid_strict(&input, "storyline_id")?;

                // 默认值必须是**合法枚举值**：此前 hint_level 默认 "low"，
                // 而合法集合里没有 low，于是新伏笔的暗示级别一直是脏数据。
                let status = match opt_enum_arg(
                    &input,
                    "status",
                    |s| fs_domain::ForeshadowingStatus::parse(s).map(|v| v.as_str()),
                    fs_domain::FORESHADOWING_STATUSES,
                )? {
                    Some(v) => v,
                    None => "Planned",
                };
                let importance = match opt_enum_arg(
                    &input,
                    "importance",
                    |s| fs_domain::ForeshadowingImportance::parse(s).map(|v| v.as_str()),
                    fs_domain::FORESHADOWING_IMPORTANCES,
                )? {
                    Some(v) => v,
                    None => "Normal",
                };
                let hint_level = match opt_enum_arg(
                    &input,
                    "hint_level",
                    |s| fs_domain::HintLevel::parse(s).map(|v| v.as_str()),
                    fs_domain::HINT_LEVELS,
                )? {
                    Some(v) => v,
                    None => "Subtle",
                };

                // 节点锚与伏笔树：先校验目标存在（表上无外键，避免写悬空 uuid）
                let (planted_node_id, payoff_node_id, parent_foreshadow_id) =
                    self.resolve_anchors(&input).await?;
                let f = self
                    .service
                    .create_foreshadow(
                        project_id,
                        name,
                        opt_str(&input, "description"),
                        status,
                        importance,
                        hint_level,
                        opt_str(&input, "hint_note"),
                        opt_str(&input, "introduced_at"),
                        opt_str(&input, "expected_reveal_at"),
                        planted_node_id,
                        payoff_node_id,
                        parent_foreshadow_id,
                        storyline_id,
                    )
                    .await?;
                Ok(json!({ "ok": true, "action": "create_foreshadow", "data": f }))
            }
            ForeshadowAction::ReviseForeshadow => {
                let id = parse_uuid(&input, "id")?;
                // 归属三态：clear_storyline=true 解除；给了 storyline_id 改挂；都没给则不动
                let clear = input
                    .get("clear_storyline")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let storyline_id = if clear {
                    Some(None)
                } else {
                    opt_uuid_strict(&input, "storyline_id")?.map(Some)
                };
                let status = opt_enum_arg(
                    &input,
                    "status",
                    |s| fs_domain::ForeshadowingStatus::parse(s).map(|v| v.as_str()),
                    fs_domain::FORESHADOWING_STATUSES,
                )?;
                let name = opt_str(&input, "name");
                let description = opt_str(&input, "description");
                let hint_note = opt_str(&input, "hint_note");
                let introduced_at = opt_str(&input, "introduced_at");
                let expected_reveal_at = opt_str(&input, "expected_reveal_at");
                let actual_reveal_at = opt_str(&input, "actual_reveal_at");
                // 节点锚与伏笔树（先校验存在）
                let (planted_node_id, payoff_node_id, parent_foreshadow_id) =
                    self.resolve_anchors(&input).await?;
                // 一个要改的字段都没给：直接报错，而不是"成功但什么都没变"
                if name.is_none()
                    && description.is_none()
                    && status.is_none()
                    && storyline_id.is_none()
                    && hint_note.is_none()
                    && introduced_at.is_none()
                    && expected_reveal_at.is_none()
                    && actual_reveal_at.is_none()
                    && planted_node_id.is_none()
                    && payoff_node_id.is_none()
                    && parent_foreshadow_id.is_none()
                {
                    anyhow::bail!(
                        "revise_foreshadow 至少要给一个要改的字段：name / description / status / hint_note / \
                         introduced_at / expected_reveal_at / actual_reveal_at / storyline_id"
                    );
                }
                let f = self
                    .service
                    .update_foreshadow(
                        id,
                        name.as_deref(),
                        description.as_deref(),
                        status,
                        hint_note.as_deref(),
                        introduced_at.as_deref(),
                        expected_reveal_at.as_deref(),
                        actual_reveal_at.as_deref(),
                        planted_node_id,
                        payoff_node_id,
                        parent_foreshadow_id,
                        storyline_id,
                    )
                    .await?;
                Ok(json!({ "ok": true, "action": "revise_foreshadow", "data": f }))
            }
            ForeshadowAction::RetireForeshadow => {
                let id = parse_uuid(&input, "id")?;
                self.service.delete_foreshadow(id).await?;
                Ok(json!({ "ok": true, "action": "retire_foreshadow", "id": id.to_string() }))
            }
            ForeshadowAction::ListForeshadows => {
                let project_id = parse_uuid(&input, "project_id")?;
                let (limit, offset) = agent::parse_page_args(&input)?;
                let (items, total) = self.service.list_foreshadows_page(project_id, limit, offset).await?;
                // 目录页投影：只给列表需要的字段（实测不投影时这些列表可到上万字符）
                let data: Vec<Value> = items
                    .iter()
                    .map(|x| agent::pick_fields(x, &["id", "name", "description", "status", "importance", "hint_level", "storyline_id", "storyline_name", "planted_node_id", "payoff_node_id", "parent_foreshadow_id"]))
                    .collect();
                Ok(agent::list_envelope("list_foreshadows", data, total, limit, offset, "伏笔", vec![]))
            }
        }
    }
}

// ============================================================
// Rule 聚合
// ============================================================

#[derive(Clone, Copy)]
pub enum RuleAction {
    CreateRule,
    ReviseRule,
    RetireRule,
    GetRule,
    ListRules,
}

pub struct RuleTool {
    action: RuleAction,
    service: Arc<RuleService>,
}

impl RuleTool {
    pub fn new(action: RuleAction, service: Arc<RuleService>) -> Self {
        Self { action, service }
    }
}

pub fn register_rule_tools(registry: &ToolRegistry, service: Arc<RuleService>) {
    for a in [
        RuleAction::CreateRule,
        RuleAction::ReviseRule,
        RuleAction::RetireRule,
        RuleAction::GetRule,
        RuleAction::ListRules,
    ] {
        registry.register(Arc::new(RuleTool::new(a, service.clone())));
    }
}

fn rule_name(a: RuleAction) -> &'static str {
    match a {
        RuleAction::CreateRule => "create_rule",
        RuleAction::ReviseRule => "revise_rule",
        RuleAction::RetireRule => "retire_rule",
        RuleAction::GetRule => "get_rule",
        RuleAction::ListRules => "list_rules",
    }
}

fn rule_description(a: RuleAction) -> String {
    match a {
        RuleAction::CreateRule => "创建世界规则（canon_rule），可指定规则级别 / 作用域 / 执行强度。".into(),
        RuleAction::ReviseRule => "修改世界规则的内容 / 级别。这会修改已有产物。".into(),
        RuleAction::RetireRule => "语义化结束一条世界规则（**即 delete_rule**，逻辑删除）。删除前请先用 get_rule / list_rules 确认目标 id。".into(),
        RuleAction::GetRule => "读取单一世界规则。".into(),
        RuleAction::ListRules => "列出某世界的全部规则，用于检索上下文。".into(),
    }
}

fn rule_schema(a: RuleAction) -> Value {
    match a {
        RuleAction::CreateRule => json!({
            "type": "object",
            "properties": {
                "world_id": { "type": "string", "description": "目标世界 UUID（先调 get_main_world 取得）" },
                "rule_content": { "type": "string" },
                "rule_level": { "type": "string", "description": "可选" },
                "affected_scope": { "type": "string", "description": "可选" },
                "enforcement": { "type": "string", "description": "可选" }
            },
            "required": ["world_id", "rule_content"]
        }),
        RuleAction::ReviseRule => json!({
            "type": "object",
            "properties": {
                "id": { "type": "string" },
                "rule_content": { "type": "string" },
                "rule_level": { "type": "string" }
            },
            "required": ["id"]
        }),
        RuleAction::RetireRule | RuleAction::GetRule => json!({
            "type": "object",
            "properties": { "id": { "type": "string" } },
            "required": ["id"]
        }),
        RuleAction::ListRules => json!({
            "type": "object",
            "properties": {
                "world_id": { "type": "string", "description": "世界 UUID" },
                "limit": { "type": "number", "description": format!("**可选**：本页条数，默认 {}，上限 {}（超过报错）", agent::LIST_DEFAULT_LIMIT, agent::LIST_MAX_LIMIT) },
                "offset": { "type": "number", "description": "**可选**：从第几条开始（默认 0）；返回里的 next_offset 就是下一页该传的值" }
            },
            "required": ["world_id"]
        }),
    }
}

#[async_trait]
impl AgentTool for RuleTool {
    fn name(&self) -> String {
        rule_name(self.action).to_string()
    }
    fn description(&self) -> String {
        rule_description(self.action)
    }
    fn input_schema(&self) -> Value {
        rule_schema(self.action)
    }

    async fn execute(&self, input: Value) -> Result<Value> {
        match self.action {
            RuleAction::CreateRule => {
                let world_id = parse_uuid(&input, "world_id")?;
                let content = opt_str(&input, "rule_content").ok_or_else(|| anyhow::anyhow!("rule_content 缺失"))?;
                let r = self
                    .service
                    .create_rule(world_id, content, opt_str(&input, "rule_level"), opt_str(&input, "affected_scope"), opt_str(&input, "enforcement"))
                    .await?;
                Ok(json!({ "ok": true, "action": "create_rule", "data": r }))
            }
            RuleAction::ReviseRule => {
                let id = parse_uuid(&input, "id")?;
                let r = self.service.update_rule(id, opt_str(&input, "rule_content"), opt_str(&input, "rule_level")).await?;
                Ok(json!({ "ok": true, "action": "revise_rule", "data": r }))
            }
            RuleAction::RetireRule => {
                let id = parse_uuid(&input, "id")?;
                self.service.delete_rule(id).await?;
                Ok(json!({ "ok": true, "action": "retire_rule", "id": id.to_string() }))
            }
            RuleAction::GetRule => {
                let id = parse_uuid(&input, "id")?;
                let r = self.service.get_rule(id).await?.ok_or_else(|| anyhow::anyhow!("规则不存在: {}", id))?;
                Ok(json!({ "ok": true, "action": "get_rule", "data": r }))
            }
            RuleAction::ListRules => {
                let world_id = parse_uuid(&input, "world_id")?;
                let (limit, offset) = agent::parse_page_args(&input)?;
                let (items, total) = self.service.list_rules_page(world_id, limit, offset).await?;
                // 目录页投影：只给列表需要的字段（实测不投影时这些列表可到上万字符）
                let data: Vec<Value> = items
                    .iter()
                    .map(|x| agent::pick_fields(x, &["id", "rule_content", "rule_level", "enforcement", "affected_scope"]))
                    .collect();
                Ok(agent::list_envelope("list_rules", data, total, limit, offset, "世界规则", vec![]))
            }
        }
    }
}

// ============================================================
// Snapshot 聚合
// ============================================================

#[derive(Clone, Copy)]
pub enum SnapshotAction {
    CreateSnapshot,
    DeleteSnapshot,
    ListSnapshots,
}

pub struct SnapshotTool {
    action: SnapshotAction,
    service: Arc<SnapshotService>,
}

impl SnapshotTool {
    pub fn new(action: SnapshotAction, service: Arc<SnapshotService>) -> Self {
        Self { action, service }
    }
}

pub fn register_snapshot_tools(registry: &ToolRegistry, service: Arc<SnapshotService>) {
    for a in [
        SnapshotAction::CreateSnapshot,
        SnapshotAction::DeleteSnapshot,
        SnapshotAction::ListSnapshots,
    ] {
        registry.register(Arc::new(SnapshotTool::new(a, service.clone())));
    }
}

fn snapshot_name(a: SnapshotAction) -> &'static str {
    match a {
        SnapshotAction::CreateSnapshot => "create_snapshot",
        SnapshotAction::DeleteSnapshot => "delete_snapshot",
        SnapshotAction::ListSnapshots => "list_snapshots",
    }
}

fn snapshot_description(a: SnapshotAction) -> String {
    match a {
        SnapshotAction::CreateSnapshot => "创建世界状态快照（故事时间 / 世界概要等可选）。".into(),
        SnapshotAction::DeleteSnapshot => {
            "**物理删除**一个快照（与 retire_* 的逻辑删除不同：快照是存档数据，删掉不留痕迹）。\
             删除前请先用 list_snapshots 确认目标 id。"
                .into()
        }
        SnapshotAction::ListSnapshots => "列出某项目的全部快照，用于检索上下文。".into(),
    }
}

fn snapshot_schema(a: SnapshotAction) -> Value {
    match a {
        SnapshotAction::CreateSnapshot => json!({
            "type": "object",
            "properties": {
                "name": { "type": "string" },
                "story_time": { "type": "string" },
                "world_summary": { "type": "string" }
            },
            "required": []
        }),
        SnapshotAction::DeleteSnapshot => json!({
            "type": "object",
            "properties": { "id": { "type": "string" } },
            "required": ["id"]
        }),
        SnapshotAction::ListSnapshots => json!({
            "type": "object",
            "properties": {
                "limit": { "type": "number", "description": format!("**可选**：本页条数，默认 {}，上限 {}（超过报错）", agent::LIST_DEFAULT_LIMIT, agent::LIST_MAX_LIMIT) },
                "offset": { "type": "number", "description": "**可选**：从第几条开始（默认 0）；返回里的 next_offset 就是下一页该传的值" }
            },
            "required": []
        }),
    }
}

#[async_trait]
impl AgentTool for SnapshotTool {
    fn name(&self) -> String {
        snapshot_name(self.action).to_string()
    }
    fn description(&self) -> String {
        snapshot_description(self.action)
    }
    fn input_schema(&self) -> Value {
        snapshot_schema(self.action)
    }

    async fn execute(&self, input: Value) -> Result<Value> {
        match self.action {
            SnapshotAction::CreateSnapshot => {
                let project_id = parse_uuid(&input, "project_id")?;
                let s = self
                    .service
                    .create_snapshot(project_id, opt_str(&input, "name"), opt_str(&input, "story_time"), opt_str(&input, "world_summary"))
                    .await?;
                Ok(json!({ "ok": true, "action": "create_snapshot", "data": s }))
            }
            SnapshotAction::DeleteSnapshot => {
                let id = parse_uuid(&input, "id")?;
                self.service.delete_snapshot(id).await?;
                Ok(json!({ "ok": true, "action": "delete_snapshot", "id": id.to_string() }))
            }
            SnapshotAction::ListSnapshots => {
                let project_id = parse_uuid(&input, "project_id")?;
                let (limit, offset) = agent::parse_page_args(&input)?;
                let (items, total) = self.service.list_snapshots_page(project_id, limit, offset).await?;
                // 目录页投影：只给列表需要的字段（实测不投影时这些列表可到上万字符）
                let data: Vec<Value> = items
                    .iter()
                    .map(|x| agent::pick_fields(x, &["id", "name", "story_time", "progress", "known_characters_count", "known_locations_count", "unresolved_foreshadows_count", "active_threads_count"]))
                    .collect();
                Ok(agent::list_envelope("list_snapshots", data, total, limit, offset, "快照", vec![]))
            }
        }
    }
}

// ============================================================
// World 聚合
// ============================================================

#[derive(Clone, Copy)]
pub enum WorldAction {
    GetMainWorld,
    UpdateMainWorld,
}

pub struct WorldTool {
    action: WorldAction,
    service: Arc<WorldService>,
}

impl WorldTool {
    pub fn new(action: WorldAction, service: Arc<WorldService>) -> Self {
        Self { action, service }
    }
}

pub fn register_world_tools(registry: &ToolRegistry, service: Arc<WorldService>) {
    for a in [WorldAction::GetMainWorld, WorldAction::UpdateMainWorld] {
        registry.register(Arc::new(WorldTool::new(a, service.clone())));
    }
}

fn world_name(a: WorldAction) -> &'static str {
    match a {
        WorldAction::GetMainWorld => "get_main_world",
        WorldAction::UpdateMainWorld => "update_main_world",
    }
}

fn world_description(a: WorldAction) -> String {
    match a {
        WorldAction::GetMainWorld => "读取项目的主世界（由 project_id 定位）。".into(),
        WorldAction::UpdateMainWorld => "修改项目主世界的基础设定（名称 / 描述 / 世界规则）。这会修改已有产物。".into(),
    }
}

fn world_schema(a: WorldAction) -> Value {
    match a {
        WorldAction::GetMainWorld => json!({
            "type": "object",
            "properties": {},
            "required": []
        }),
        WorldAction::UpdateMainWorld => json!({
            "type": "object",
            "properties": {
                "name": { "type": "string" },
                "description": { "type": "string" },
                "world_rules": { "type": "string" }
            },
            "required": []
        }),
    }
}

#[async_trait]
impl AgentTool for WorldTool {
    fn name(&self) -> String {
        world_name(self.action).to_string()
    }
    fn description(&self) -> String {
        world_description(self.action)
    }
    fn input_schema(&self) -> Value {
        world_schema(self.action)
    }

    async fn execute(&self, input: Value) -> Result<Value> {
        match self.action {
            WorldAction::GetMainWorld => {
                let project_id = parse_uuid(&input, "project_id")?;
                let w: World = self
                    .service
                    .get_main_world(project_id)
                    .await?
                    .ok_or_else(|| anyhow::anyhow!("主世界不存在: {}", project_id))?;
                Ok(json!({ "ok": true, "action": "get_main_world", "data": serde_json::to_value(w)? }))
            }
            WorldAction::UpdateMainWorld => {
                let project_id = parse_uuid(&input, "project_id")?;
                let w: World = self
                    .service
                    .update_main_world(project_id, opt_str(&input, "name"), opt_str(&input, "description"), opt_str(&input, "world_rules"))
                    .await?;
                Ok(json!({ "ok": true, "action": "update_main_world", "data": serde_json::to_value(w)? }))
            }
        }
    }
}

// ============================================================
// Project 聚合（项目级元数据：premise 等）
// ============================================================
//
// premise 是项目级元数据，与 genre 同性质；存放在 `project` 表而非 `world.config`，
// 原因：premise 在 world 尚未存在时就需要有，且未来多 world 共享同一 premise。
//
// 不走 MutationCommitter（项目元数据不在 Canon 概念内），直接走 ProjectService。
// 见 docs/ROADMAP.md「项目级脑洞字段」设计与 P2 工具清单。

#[derive(Clone, Copy)]
pub enum ProjectAction {
    /// 读取项目（含 premise）。用以让 Agent 在重载对话时取回上次的脑洞版本。
    GetProject,
    /// 修改项目元数据（name / description / premise）。premise 是脑洞写入点。
    UpdateProject,
    /// 列出项目（用于跨项目导航；本工具主要服务于 Agent 提示词中"当前项目"上下文）。
    ListProjects,
}

pub struct ProjectTool {
    action: ProjectAction,
    service: Arc<ProjectService>,
}

impl ProjectTool {
    pub fn new(action: ProjectAction, service: Arc<ProjectService>) -> Self {
        Self { action, service }
    }
}

pub fn register_project_tools(registry: &ToolRegistry, service: Arc<ProjectService>) {
    for a in [
        ProjectAction::GetProject,
        ProjectAction::UpdateProject,
        ProjectAction::ListProjects,
    ] {
        registry.register(Arc::new(ProjectTool::new(a, service.clone())));
    }
}

fn project_name(a: ProjectAction) -> &'static str {
    match a {
        ProjectAction::GetProject => "get_project",
        ProjectAction::UpdateProject => "update_project",
        ProjectAction::ListProjects => "list_projects",
    }
}

fn project_description(a: ProjectAction) -> String {
    match a {
        ProjectAction::GetProject => "读取项目元数据（含 premise / 脑洞）。".into(),
        ProjectAction::UpdateProject => "修改项目元数据。premise 字段是故事脑洞/前提的写入点，\
            用于在用户与 Agent 持续沟通打磨后由用户确认落库；写入后作为后续所有世界观、\
            角色、叙事生成的最强 prompt 约束。注意：premise 应在用户明确同意后才写入，\
            Agent 不要主动覆盖用户已确认的脑洞。"
            .into(),
        ProjectAction::ListProjects => {
            "列出项目（**目录页**：id / 名称 / 状态 / 更新时间，不含 premise / config 全文）。\
             默认返回 20 条并给出 total；用 limit / offset 翻页。需要某个项目的完整设定用 get_project。"
                .into()
        }
    }
}

fn project_schema(a: ProjectAction) -> Value {
    match a {
        ProjectAction::GetProject => json!({
            "type": "object",
            "properties": {
                "project_id": { "type": "string" }
            },
            "required": ["project_id"]
        }),
        ProjectAction::UpdateProject => json!({
            "type": "object",
            "properties": {
                "name": { "type": "string" },
                "description": { "type": "string" },
                "premise": { "type": "string" }
            },
            "required": []
        }),
        ProjectAction::ListProjects => json!({
            "type": "object",
            "properties": {
                "limit": { "type": "number", "description": format!("**可选**：本页条数，默认 {}，上限 {}（超过报错）。列表只给目录字段（id / 名称 / 状态 / 更新时间）", agent::LIST_DEFAULT_LIMIT, agent::LIST_MAX_LIMIT) },
                "offset": { "type": "number", "description": "**可选**：从第几条开始（默认 0）；返回里的 next_offset 就是下一页该传的值" }
            },
            "required": []
        }),
    }
}

#[async_trait]
impl AgentTool for ProjectTool {
    fn name(&self) -> String {
        project_name(self.action).to_string()
    }
    fn description(&self) -> String {
        project_description(self.action)
    }
    fn input_schema(&self) -> Value {
        project_schema(self.action)
    }

    async fn execute(&self, input: Value) -> Result<Value> {
        match self.action {
            ProjectAction::GetProject => {
                let project_id = parse_uuid(&input, "project_id")?;
                let p = self
                    .service
                    .get_project(project_id)
                    .await?
                    .ok_or_else(|| anyhow::anyhow!("项目不存在: {}", project_id))?;
                Ok(json!({ "ok": true, "action": "get_project", "data": p }))
            }
            ProjectAction::UpdateProject => {
                let project_id = parse_uuid(&input, "project_id")?;
                let p = self
                    .service
                    .update_project(
                        project_id,
                        opt_str(&input, "name"),
                        opt_str(&input, "description"),
                        None,
                        opt_str(&input, "premise"),
                    )
                    .await?;
                Ok(json!({ "ok": true, "action": "update_project", "data": p }))
            }
            ProjectAction::ListProjects => {
                let (limit, offset) = agent::parse_page_args(&input)?;
                let (items, total) = self.service.list_projects_page(limit, offset).await?;
                // 目录页投影：实测不投影时 1172 个项目 = 425,018 字符（每个都带 config 全文）。
                let data: Vec<Value> = items
                    .iter()
                    .map(|p| agent::pick_fields(p, &["id", "name", "status", "updated_at"]))
                    .collect();
                Ok(agent::list_envelope(
                    "list_projects",
                    data,
                    total,
                    limit,
                    offset,
                    "项目",
                    vec![],
                ))
            }
        }
    }
}

// ============================================================
// History 聚合（仅创建与读取；历史不可篡改，不提供修改 / 删除）
// ============================================================

#[derive(Clone, Copy)]
pub enum HistoryAction {
    CreateEvent,
    /// 修改事件：此前事件只能建、不能改，结构化字段更是写不进去
    ReviseEvent,
    /// 语义化结束事件（逻辑删除）：此前事件连删除路径都没有
    RetireEvent,
    CreateFact,
    /// 语义化结束事实（逻辑删除）
    RetireFact,
    ListEvents,
    ListFacts,
}

pub struct HistoryTool {
    action: HistoryAction,
    service: Arc<HistoryService>,
}

impl HistoryTool {
    pub fn new(action: HistoryAction, service: Arc<HistoryService>) -> Self {
        Self { action, service }
    }
}

pub fn register_history_tools(registry: &ToolRegistry, service: Arc<HistoryService>) {
    for a in [
        HistoryAction::CreateEvent,
        HistoryAction::ReviseEvent,
        HistoryAction::RetireEvent,
        HistoryAction::CreateFact,
        HistoryAction::RetireFact,
        HistoryAction::ListEvents,
        HistoryAction::ListFacts,
    ] {
        registry.register(Arc::new(HistoryTool::new(a, service.clone())));
    }
}

fn history_name(a: HistoryAction) -> &'static str {
    match a {
        HistoryAction::CreateEvent => "create_event",
        HistoryAction::ReviseEvent => "revise_event",
        HistoryAction::RetireEvent => "retire_event",
        HistoryAction::CreateFact => "create_fact",
        HistoryAction::RetireFact => "retire_fact",
        HistoryAction::ListEvents => "list_events",
        HistoryAction::ListFacts => "list_facts",
    }
}

fn history_description(a: HistoryAction) -> String {
    match a {
        HistoryAction::CreateEvent => {
            "创建一条历史事件（Canon 历史记录）。**建议填结构化字段**而不是把什么都塞进              description：发生时间 when、地点 where、参与方 participants（实体 id 数组）、             直接后果 consequences、揭示时机 reveal_at——填了之后才能按地点/暗线检索事件。"
                .into()
        }
        HistoryAction::ReviseEvent => {
            "修改已有事件（含结构化字段）。不传的字段保持原值；             attributes 内的键（where / participants / consequences / reveal_at）按传入的键逐个更新，             未传的键保持原值。"
                .into()
        }
        HistoryAction::RetireEvent => {
            "语义化结束一条事件（逻辑删除，`status` 置为 Deleted，数据保留可追溯）。             用于「建错了要撤回」。结束前先用 list_events 确认 id；不要期望它被物理删除。"
                .into()
        }
        HistoryAction::RetireFact => {
            "语义化结束一条事实（`status` 置为 Retired，数据保留）。             用于撤回写错的事实。结束后 list_facts 不再返回它。"
                .into()
        }
        HistoryAction::CreateFact => "创建一条事实（Fact，含确定性等级）。".into(),
        HistoryAction::ListEvents => {
            "列出某项目的历史事件（可按 limit 限制条数），返回含结构化字段             （event_type / event_time / attributes.where / participants / consequences / reveal_at）。"
                .into()
        }
        HistoryAction::ListFacts => "列出某项目的全部事实，用于检索上下文。".into(),
    }
}

fn history_schema(a: HistoryAction) -> Value {
    match a {
        HistoryAction::CreateEvent => json!({
            "type": "object",
            "properties": {
                "name": { "type": "string" },
                "description": { "type": "string", "description": "事件说明（叙述性内容）" },
                "event_type": { "type": "string", "description": "可选：事件类型，如 灾难 / 战争 / 失踪 / 崛起 / 转折" },
                "when": { "type": "string", "description": "发生时间（如「千年之前」「第二卷·中年」）。请填这里而不是塞进 description" },
                "era_order": { "type": "number", "description": "可选：**历史轴排序锚**（越小越早）。when 给人看、这个给排序用——中文时间没法比大小，没它历史轴排不对" },
                "duration": { "type": "string", "description": "可选：持续时间" },
                "where": { "type": "string", "description": "发生地点（叙述，如「全世界」「阴面·轮回渡」）" },
                "participants": {
                    "type": "array",
                    "description": "参与方（相关实体）。填了才能从事件反查实体",
                    "items": {
                        "type": "object",
                        "properties": {
                            "entity_id": { "type": "string", "description": "实体 UUID（先用 list_entities 取得）" },
                            "name": { "type": "string", "description": "该实体在事件中的称谓（可省）" },
                            "role": { "type": "string", "description": "在事件中的角色（如 失踪者 / 主导者 / 受害者）" }
                        },
                        "required": ["entity_id"]
                    }
                },
                "consequences": { "type": "string", "description": "直接后果（可多条，用换行分隔）" },
                "reveal_at": { "type": "string", "description": "揭示时机（如「暗线，前期不透」「第三卷揭露」）" },
                "narrative_node_id": { "type": "string", "description": "**可选**：这件事发生在哪个叙事节点（章 / 场）——填了才能和细纲对上，也才能按节点回溯「这一章发生了什么」。先用 list_nodes 拿 id" }
            },
            "required": ["name", "description"]
        }),
        HistoryAction::ReviseEvent => json!({
            "type": "object",
            "properties": {
                "id": { "type": "string", "description": "事件 UUID（先用 list_events 取得）" },
                "era_order": { "type": "number", "description": "历史轴排序锚（越小越早；不传则保持原值）" },
                "name": { "type": "string" },
                "description": { "type": "string" },
                "event_type": { "type": "string" },
                "when": { "type": "string", "description": "发生时间" },
                "duration": { "type": "string" },
                "where": { "type": "string" },
                "participants": {
                    "type": "array",
                    "description": "参与方（按传入整体替换）",
                    "items": {
                        "type": "object",
                        "properties": {
                            "entity_id": { "type": "string" },
                            "name": { "type": "string" },
                            "role": { "type": "string" }
                        },
                        "required": ["entity_id"]
                    }
                },
                "consequences": { "type": "string" },
                "reveal_at": { "type": "string" },
                "narrative_node_id": { "type": "string", "description": "**可选**：改挂到另一个叙事节点（章 / 场）" },
                "clear_narrative_node": { "type": "boolean", "description": "**可选**：解绑叙事节点（与 narrative_node_id 互斥）" }
            },
            "required": ["id"]
        }),
        HistoryAction::RetireEvent => json!({
            "type": "object",
            "properties": {
                "id": { "type": "string", "description": "事件 UUID（先用 list_events 取得）" }
            },
            "required": ["id"]
        }),
        HistoryAction::RetireFact => json!({
            "type": "object",
            "properties": {
                "id": { "type": "string", "description": "事实 UUID（先用 list_facts 取得）" }
            },
            "required": ["id"]
        }),
        HistoryAction::CreateFact => json!({
            "type": "object",
            "properties": {
                "content": { "type": "string" },
                "category": { "type": "string", "description": "可选" },
                "certainty": { "type": "string", "description": "如 CANON / SPECULATIVE" }
            },
            "required": ["content", "certainty"]
        }),
        HistoryAction::ListEvents => json!({
            "type": "object",
            "properties": {
                "order_by": { "type": "string", "enum": ["recent", "era"], "description": "可选，默认 recent（最近优先）；传 era 按**历史轴**排序（依赖 era_order，未标定的排最后）" },
                "limit": { "type": "number", "description": format!("**可选**：本页条数，默认 {}，上限 {}（超过报错）", agent::LIST_DEFAULT_LIMIT, agent::LIST_MAX_LIMIT) },
                "offset": { "type": "number", "description": "**可选**：从第几条开始（默认 0）；返回里的 next_offset 就是下一页该传的值" }
            },
            "required": []
        }),
        HistoryAction::ListFacts => json!({
            "type": "object",
            "properties": {
                "limit": { "type": "number", "description": format!("**可选**：本页条数，默认 {}，上限 {}（超过报错）", agent::LIST_DEFAULT_LIMIT, agent::LIST_MAX_LIMIT) },
                "offset": { "type": "number", "description": "**可选**：从第几条开始（默认 0）；返回里的 next_offset 就是下一页该传的值" }
            },
            "required": []
        }),
    }
}

#[async_trait]
impl AgentTool for HistoryTool {
    fn name(&self) -> String {
        history_name(self.action).to_string()
    }
    fn description(&self) -> String {
        history_description(self.action)
    }
    fn input_schema(&self) -> Value {
        history_schema(self.action)
    }

    async fn execute(&self, input: Value) -> Result<Value> {
        match self.action {
            HistoryAction::CreateEvent => {
                let project_id = parse_uuid(&input, "project_id")?;
                let name = opt_str(&input, "name").ok_or_else(|| anyhow::anyhow!("name 缺失"))?;
                let desc = opt_str(&input, "description").ok_or_else(|| anyhow::anyhow!("description 缺失"))?;
                let attributes = application::history_service::collect_event_attributes(&input)?;
                let e = self
                    .service
                    .create_event(
                        project_id,
                        name,
                        desc,
                        opt_str(&input, "event_type"),
                        opt_str(&input, "when"),
                        opt_str(&input, "duration"),
                        &attributes,
                        opt_i64_strict(&input, "era_order")?,
                        opt_uuid_strict(&input, "narrative_node_id")?,
                    )
                    .await?;
                Ok(json!({ "ok": true, "action": "create_event", "data": e }))
            }
            HistoryAction::ReviseEvent => {
                let id = parse_uuid(&input, "id")?;
                // attributes 只在「本次确实传了相关键」时才提交，
                // 否则会把库里已有的 where / participants 清空
                let attributes = application::history_service::collect_event_attributes(&input)?;
                let attributes = if attributes
                    .as_object()
                    .map(|o| o.is_empty())
                    .unwrap_or(true)
                {
                    None
                } else {
                    Some(attributes)
                };
                let e = self
                    .service
                    .update_event(
                        id,
                        opt_str_owned(&input, "name").as_deref(),
                        opt_str_owned(&input, "description").as_deref(),
                        opt_str_owned(&input, "event_type").as_deref(),
                        opt_str_owned(&input, "when").as_deref(),
                        opt_str_owned(&input, "duration").as_deref(),
                        attributes.as_ref(),
                        opt_i64_strict(&input, "era_order")?,
                        opt_uuid_strict(&input, "narrative_node_id")?,
                        opt_bool(&input, "clear_narrative_node")?,
                    )
                    .await?;
                Ok(json!({ "ok": true, "action": "revise_event", "data": e }))
            }
            HistoryAction::RetireEvent => {
                let id = parse_uuid(&input, "id")?;
                self.service.delete_event(id).await?;
                Ok(json!({ "ok": true, "action": "retire_event", "id": id.to_string() }))
            }
            HistoryAction::RetireFact => {
                let id = parse_uuid(&input, "id")?;
                self.service.delete_fact(id).await?;
                Ok(json!({ "ok": true, "action": "retire_fact", "id": id.to_string() }))
            }
            HistoryAction::CreateFact => {
                let project_id = parse_uuid(&input, "project_id")?;
                let content = opt_str(&input, "content").ok_or_else(|| anyhow::anyhow!("content 缺失"))?;
                let certainty = opt_str(&input, "certainty").ok_or_else(|| anyhow::anyhow!("certainty 缺失"))?;
                let f = self.service.create_fact(project_id, content, opt_str(&input, "category"), certainty).await?;
                Ok(json!({ "ok": true, "action": "create_fact", "data": f }))
            }
            HistoryAction::ListEvents => {
                let project_id = parse_uuid(&input, "project_id")?;
                let (limit, offset) = agent::parse_page_args(&input)?;
                // 排序锚：`when` 是给人看的中文自由文本（「远昔」「缓变」），没法比大小，
                // 所以「按历史轴看」要显式传 order_by="era"（依赖 era_order 数值锚）。
                let order_by = opt_str(&input, "order_by").unwrap_or("recent");
                let (items, total) = self
                    .service
                    .list_events_page_ordered(project_id, limit, offset, order_by.trim())
                    .await?;
                // 目录页投影：只给列表需要的字段（实测不投影时这些列表可到上万字符）
                let data: Vec<Value> = items
                    .iter()
                    .map(|x| agent::pick_fields(x, &["id", "name", "event_type", "event_time", "duration", "description", "narrative_node_id", "narrative_node_title"]))
                    .collect();
                Ok(agent::list_envelope("list_events", data, total, limit, offset, "历史事件", vec![]))
            }
            HistoryAction::ListFacts => {
                let project_id = parse_uuid(&input, "project_id")?;
                let (limit, offset) = agent::parse_page_args(&input)?;
                let (items, total) = self.service.list_facts_page(project_id, limit, offset).await?;
                // 目录页投影：只给列表需要的字段（实测不投影时这些列表可到上万字符）
                let data: Vec<Value> = items
                    .iter()
                    .map(|x| agent::pick_fields(x, &["id", "content", "category", "certainty", "status"]))
                    .collect();
                Ok(agent::list_envelope("list_facts", data, total, limit, offset, "事实", vec![]))
            }
        }
    }
}

// ============================================================
// 统一注册入口（Phase A + B）
// ============================================================

/// 在组合根构造所有聚合的 application service，并把全部领域工具注册进 `registry`。
///
/// 调用方（main.rs）只需传入已建好的 `PgPool`；各 service 的 repo / committer /
/// resolver 在此统一构造，与 `api/*` handler 中的 `service()` 同构。
pub fn register_all_domain_tools(registry: &ToolRegistry, pool: &PgPool) {
    let committer = Arc::new(MutationCommitter::new(Arc::new(DbMutationCommitter::new(pool.clone()))));
    let resolver = Arc::new(DbProjectResolverPort::new(pool.clone()));

    let entity = Arc::new(EntityService::new(
        Arc::new(DbEntityRepositoryPort::new(pool.clone())),
        committer.clone(),
        resolver.clone(),
        "ai",
    ));
    register_entity_tools(registry, entity.clone());
    // 地点档案（设计档案 + 身份属性）：创建地点后由 AI 补全，否则详情面板永远是空的
    register_location_profile_tools(registry, entity.clone());
    register_golden_finger_tools(registry, entity.clone());
    register_faction_profile_tools(registry, entity.clone());
    register_character_tools(registry, entity.clone());
    registry.register(Arc::new(BulkLocationProfileTool::new(entity.clone())));
    registry.register(Arc::new(BulkCharacterProfileTool::new(entity.clone())));

    let narrative = Arc::new(NarrativeService::new(
        Arc::new(DbNarrativeRepositoryPort::new(pool.clone())),
        committer.clone(),
        resolver.clone(),
    ));
    register_narrative_tools(registry, narrative.clone());

    let storyline = Arc::new(StorylineService::new(Arc::new(DbStorylineRepositoryPort::new(pool.clone()))));
    register_storyline_tools(registry, storyline.clone());

    let foreshadow = Arc::new(ForeshadowService::new(Arc::new(DbForeshadowRepositoryPort::new(pool.clone()))));
    register_foreshadow_tools(registry, foreshadow.clone(), pool.clone());

    let rule = Arc::new(RuleService::new(Arc::new(DbRuleRepositoryPort::new(pool.clone()))));
    register_rule_tools(registry, rule);

    let snapshot = Arc::new(SnapshotService::new(
        Arc::new(DbSnapshotRepositoryPort::new(pool.clone())),
        Arc::new(DbNarrativeStateWritePort::new(pool.clone())),
    ));
    register_snapshot_tools(registry, snapshot);

    let world = Arc::new(WorldService::new(Arc::new(DbWorldRepositoryPort::new(pool.clone()))));
    register_world_tools(registry, world.clone());

    let project = Arc::new(ProjectService::new(
        Arc::new(DbProjectRepositoryPort::new(pool.clone())),
        world.clone(),
    ));
    register_project_tools(registry, project.clone());

    let history = Arc::new(HistoryService::new(Arc::new(DbHistoryRepositoryPort::new(pool.clone()))));
    register_history_tools(registry, history.clone());

    // 轻量全景索引：把「项目进行到什么情况」压成一次 ≤3K 字符的调用。
    // 原先这个问题靠 batch_call 批量拉全量，实测一次产生 78,616 字符的工具结果。
    registry.register(Arc::new(ProjectIndexTool::new(
        project, entity, world, narrative, storyline, foreshadow, history,
    )));

    // 引导推进工具（confirm_step）：纯 pool + agent::guide::validate_step，
    // 不依赖任何 service（避免把"推进阶段"耦合到任何具体业务层）。
    register_guide_tools(registry, pool.clone());
}

// ============================================================
// Guide（引导推进）工具
// ============================================================

/// 组装「引导阶段校验」所需的项目快照。
///
/// `confirm_step` 与 `get_project_status` 共用：两者要判断的正是同一组产物
/// （premise / 世界观规则 / 各类实体数 / 剧情线挂载），各写一份必然会漂移。
///
/// **所有 SQL 失败都向上报错**：查不到数据只意味着"校验函数看不到"，
/// 若静默当成 0，用户点「确认推进」会看到"还差 3 个地点"——而他其实有 5 个。
/// 这种假阴性比直接报错难查得多。
async fn project_status_snapshot(
    pool: &PgPool,
    project_id: Uuid,
) -> Result<(MinCompleteSnapshot, Value)> {
    let project_row: Option<(Option<String>, Value)> = sqlx::query_as(
        "SELECT premise, COALESCE(config, '{}'::jsonb) FROM project WHERE id = $1",
    )
    .bind(project_id)
    .fetch_optional(pool)
    .await
    .context("读取项目前提 / config 失败")?;
    let (premise, config) =
        project_row.ok_or_else(|| anyhow::anyhow!("项目不存在: {}", project_id))?;

    let mut snapshot = MinCompleteSnapshot::default();
    snapshot.project_premise = premise;

    let world_row: Option<(Uuid, Option<String>)> = sqlx::query_as(
        "SELECT id, description FROM world WHERE project_id = $1 AND is_main = true LIMIT 1",
    )
    .bind(project_id)
    .fetch_optional(pool)
    .await
    .context("读取主世界失败")?;

    if let Some((world_id, world_desc)) = world_row {
        snapshot.world_description = world_desc;
        let (rule_count,): (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM canon_rule WHERE world_id = $1")
                .bind(world_id)
                .fetch_one(pool)
                .await
                .context("统计世界规则数失败")?;
        snapshot.world_rule_entity_count = rule_count;
    }

    // 各类实体数量（按 entity_type.name 筛，排除逻辑删除）
    for (kind_name, counter) in &[
        ("Location", "location_entity_count"),
        ("Faction", "faction_entity_count"),
        ("Item", "item_entity_count"),
        ("Character", "character_entity_count"),
        ("golden_finger", "golden_finger_entity_count"),
    ] {
        let (n,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM entity e \
             JOIN entity_type et ON et.id = e.entity_type_id \
             WHERE e.project_id = $1 AND et.name = $2 AND e.status != 'Deleted'",
        )
        .bind(project_id)
        .bind(*kind_name)
        .fetch_one(pool)
        .await
        .with_context(|| format!("统计 {} 实体数失败", kind_name))?;
        match *counter {
            "location_entity_count" => snapshot.location_entity_count = n,
            "faction_entity_count" => snapshot.faction_entity_count = n,
            "item_entity_count" => snapshot.item_entity_count = n,
            "character_entity_count" => snapshot.character_entity_count = n,
            "golden_finger_entity_count" => snapshot.golden_finger_entity_count = n,
            other => anyhow::bail!("未知的实体计数目标：{}", other),
        }
    }

    // 金手指是否已与角色建立关系（拥有 → 主角）
    let (has_rel,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM relation r \
         JOIN entity src ON src.id = r.source_entity_id \
         JOIN entity_type src_et ON src_et.id = src.entity_type_id \
         JOIN entity dst ON dst.id = r.target_entity_id \
         JOIN entity_type dst_et ON dst_et.id = dst.entity_type_id \
         WHERE r.project_id = $1 AND r.valid_until IS NULL \
         AND ((src_et.name = 'golden_finger' AND dst_et.name = 'Character') \
           OR (src_et.name = 'Character' AND dst_et.name = 'golden_finger'))",
    )
    .bind(project_id)
    .fetch_one(pool)
    .await
    .context("统计金手指关系失败")?;
    snapshot.golden_finger_has_relation_to_protagonist = has_rel > 0;

    // 剧情线：主线按 importance = 'Main' 判定（非 Main 即副线）
    let storyline_rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT id::text, COALESCE(importance, 'Normal') FROM storyline WHERE project_id = $1",
    )
    .bind(project_id)
    .fetch_all(pool)
    .await
    .context("统计剧情线失败")?;
    snapshot.storyline_total_count = storyline_rows.len() as i64;
    let main_count = storyline_rows
        .iter()
        .filter(|(_, importance)| importance == "Main")
        .count() as i64;
    snapshot.main_storyline_count = main_count;
    snapshot.sub_storyline_count = (snapshot.storyline_total_count - main_count).max(0);

    // 副线挂载数：子节点非主线且父节点存在
    let (attached_count,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM storyline_relation r \
         WHERE r.project_id = $1 \
         AND EXISTS (SELECT 1 FROM storyline c WHERE c.id = r.child_id AND c.importance != 'Main')",
    )
    .bind(project_id)
    .fetch_one(pool)
    .await
    .context("统计副线挂载数失败")?;
    snapshot.attached_storyline_count = attached_count;

    // 细纲：卷节点数 + 还没有节点挂靠的重要故事线
    let (volume_node_count,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM narrative_node WHERE project_id = $1 AND node_type = 'Volume' AND status != 'Deleted'",
    )
    .bind(project_id)
    .fetch_one(pool)
    .await
    .context("统计卷节点数失败")?;
    snapshot.volume_node_count = volume_node_count;

    let (important_without_node,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM storyline s WHERE s.project_id = $1 AND s.importance IN ('Main', 'Important') AND NOT EXISTS (SELECT 1 FROM narrative_node n WHERE n.storyline_id = s.id AND n.status != 'Deleted')",
    )
    .bind(project_id)
    .fetch_one(pool)
    .await
    .context("统计未挂节点的重要故事线失败")?;
    snapshot.important_storylines_without_node = important_without_node;

    // 细纲·章表：章节点数、空壳弧（下面一个章都没有的弧）、空壳章（没有一句话事件）
    let (chapter_node_count,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM narrative_node WHERE project_id = $1 AND node_type = 'Chapter' AND status != 'Deleted'",
    )
    .bind(project_id)
    .fetch_one(pool)
    .await
    .context("统计章节点数失败")?;
    snapshot.chapter_node_count = chapter_node_count;

    let (arcs_without_chapter,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM narrative_node arc \
         WHERE arc.project_id = $1 AND arc.node_type = 'Arc' AND arc.status != 'Deleted' \
         AND NOT EXISTS (SELECT 1 FROM narrative_node ch \
                         WHERE ch.parent_id = arc.id AND ch.node_type = 'Chapter' AND ch.status != 'Deleted')",
    )
    .bind(project_id)
    .fetch_one(pool)
    .await
    .context("统计空壳弧失败")?;
    snapshot.arcs_without_chapter = arcs_without_chapter;

    let (chapters_without_description,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM narrative_node \
         WHERE project_id = $1 AND node_type = 'Chapter' AND status != 'Deleted' \
         AND (description IS NULL OR btrim(description) = '')",
    )
    .bind(project_id)
    .fetch_one(pool)
    .await
    .context("统计空壳章失败")?;
    snapshot.chapters_without_description = chapters_without_description;

    // 细纲·场景：场景数 + 必填属性不全的场景数。
    // 字段清单来自 agent::guide::SCENE_REQUIRED_FIELDS（单一事实源），
    // 不在这里再抄一份——两处清单必然分叉。
    let (scene_node_count,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM narrative_node WHERE project_id = $1 AND node_type = 'Scene' AND status != 'Deleted'",
    )
    .bind(project_id)
    .fetch_one(pool)
    .await
    .context("统计场景节点数失败")?;
    snapshot.scene_node_count = scene_node_count;

    let scene_field_checks: Vec<String> = agent::guide::SCENE_REQUIRED_FIELDS
        .iter()
        .map(|field| format!("COALESCE(btrim(attributes->>'{}'), '') <> ''", field))
        .collect();
    let scene_missing_sql = format!(
        "SELECT COUNT(*) FROM narrative_node \
         WHERE project_id = $1 AND node_type = 'Scene' AND status != 'Deleted' \
         AND NOT ({})",
        scene_field_checks.join(" AND ")
    );
    let (scenes_missing_required_fields,): (i64,) = sqlx::query_as(&scene_missing_sql)
        .bind(project_id)
        .fetch_one(pool)
        .await
        .context("统计必填属性不全的场景失败")?;
    snapshot.scenes_missing_required_fields = scenes_missing_required_fields;

    Ok((snapshot, config))
}

//
// 单一动作 confirm_step：用户在前端点"确认推进"按钮触发（不是 agent 调用）。
// 工具内部从 DB 查 MinCompleteSnapshot → 调 agent::guide::validate_step
// → 校验通过则把 project.config.current_step 推进到 next；失败则返回结构化报告。
//
// 这个工具是「引导阶段」的事实落点；与所有 service 解耦以避免引入循环依赖。

use agent::guide::{validate_step, MinCompleteSnapshot, ValidationReport};
use anyhow::Context;

pub struct GuideTool {
    pool: PgPool,
}

impl GuideTool {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

pub fn register_guide_tools(registry: &ToolRegistry, pool: PgPool) {
    registry.register(Arc::new(GuideTool::new(pool.clone())));
    // 只读的进度查询：与 confirm_step 共用同一套快照与校验，
    // 但**绝不推进**任何东西（问进度不该有副作用）。
    registry.register(Arc::new(ProjectStatusTool::new(pool.clone())));
    // 细纲细化度：引导进度只回答"阶段走到哪了"，回答不了"细纲细到什么程度、
    // 下一步该做哪个弧"。没有它，AI 在 100+ 章规模下无法自查（列表投影不含 description）。
    registry.register(Arc::new(OutlineProgressTool::new(pool)));
}

/// 项目引导进度查询（只读）。
///
/// 存在的理由：以前只有「注入到 prompt 的当前阶段文本」这一条线索，
/// 被问「我们现在到哪一步了」时只能含糊作答。这个工具把「当前阶段 + 每一步
/// 到底缺什么」摆出来，答案与 confirm_step 的判定完全一致（共用快照与 validate_step），
/// 不会出现「这里说就绪、点按钮却报缺东西」。
pub struct ProjectStatusTool {
    pool: PgPool,
}

impl ProjectStatusTool {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl AgentTool for ProjectStatusTool {
    fn name(&self) -> String {
        "get_project_status".to_string()
    }

    fn description(&self) -> String {
        "查询本项目当前的引导进度：当前阶段、每个阶段的就绪情况、以及未就绪阶段**还缺什么**。         只读，不会推进阶段（推进要由前端用户点按钮）。         被问「我们现在到哪一步了 / 还差什么」时用它作答，不要凭印象猜。"
            .to_string()
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "project_id": { "type": "string", "description": "项目 UUID（框架会自动注入当前会话的项目）" }
            },
            "required": ["project_id"]
        })
    }

    async fn execute(&self, input: Value) -> Result<Value> {
        let project_id = parse_uuid(&input, "project_id")?;
        let mut report = guide_status(&self.pool, project_id).await?;
        report["ok"] = json!(true);
        report["action"] = json!("get_project_status");
        Ok(report)
    }
}

/// 细纲细化度查询（只读）。
///
/// 存在的理由：`list_nodes` 的目录投影里**没有 description**，AI 想找出「哪一章还是空壳」
/// 只能逐节点 `get_node`（单轮工具迭代上限 50 次、列表每页 20 条），
/// 在 100+ 章的规模下根本做不完——于是它只能凭印象说"细纲差不多了"。
/// 这个工具把「细到什么程度了 + 下一步该做哪一块」压成一次调用。
///
/// 判定口径与 `beats.chapters` / `beats.scenes` 两步的 `validate_step` 保持一致
/// （章要有 description、场景要有 [`agent::guide::SCENE_REQUIRED_FIELDS`]），
/// 否则又会出现「这里说齐了、点推进却报缺东西」。
pub struct OutlineProgressTool {
    pool: PgPool,
}

impl OutlineProgressTool {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

/// 树序节点行：(id, title, node_type, parent_id, description, attributes, 树序路径)
type OutlineRow = (
    Uuid,
    String,
    String,
    Option<Uuid>,
    Option<String>,
    Value,
    Vec<i32>,
);

/// 场景的 `attributes` 是否具备全部必填字段。
///
/// 与 `beats.scenes` 步的 `validate_step` 是**同一口径**：字段清单同取自
/// [`agent::guide::SCENE_REQUIRED_FIELDS`]，判空方式等价于 SQL 端的
/// `btrim(attributes->>'x') <> ''`（这几个字段在数据里都是字符串）。
/// 口径一旦分叉，就会出现「工具说可以开始写正文了，点推进却报场景缺字段」。
fn scene_attributes_ready(attributes: &Value) -> bool {
    agent::guide::SCENE_REQUIRED_FIELDS.iter().all(|field| {
        attributes
            .get(*field)
            .and_then(|v| v.as_str())
            .map(|s| !s.trim().is_empty())
            .unwrap_or(false)
    })
}

/// 列表型字段最多返回多少条（工具结果有 12,000 字符硬上限，
/// 章节多的项目必须截断；总数另行给出，不丢信息）。
const OUTLINE_LIST_MAX: usize = 20;

#[async_trait]
impl AgentTool for OutlineProgressTool {
    fn name(&self) -> String {
        "outline_progress".to_string()
    }

    fn description(&self) -> String {
        "查询本项目「细纲细化到什么程度」：卷/弧/章/场各多少个、哪些章还是空壳（没有一句话事件）、\
         哪些弧下面还没有章、哪些重要故事线还没挂到节点、哪些章还没有场景，\
         以及**下一步该做哪一块**（next_suggestion）。\
         被问「细纲还差什么 / 接下来做什么 / 从哪开始」时先用它；\
         不要靠 list_nodes 逐页翻去找空壳章——列表投影里没有 description，翻不出来。\
         只读，不改任何数据。"
            .to_string()
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "project_id": { "type": "string", "description": "项目 UUID（框架会自动注入当前会话的项目）" }
            },
            "required": ["project_id"]
        })
    }

    async fn execute(&self, input: Value) -> Result<Value> {
        let project_id = parse_uuid(&input, "project_id")?;
        let mut report = outline_progress_report(&self.pool, project_id).await?;
        report["ok"] = json!(true);
        report["action"] = json!("outline_progress");
        Ok(report)
    }
}

/// 组装细纲进度报告。一次递归查询把整棵树按**树序**取出，其余统计在内存里算。
///
/// 为什么必须按树序：`narrative_node.sort_order` 是**同父内**的序号，
/// 直接 `ORDER BY sort_order` 会让不同弧的章交错出现（第 1 章、第 4 章、第 7 章……），
/// 而"接下来该写哪几章"完全依赖顺序正确。
async fn outline_progress_report(pool: &PgPool, project_id: Uuid) -> Result<Value> {
    let rows: Vec<OutlineRow> = sqlx::query_as(
        "WITH RECURSIVE tree AS ( \
             SELECT id, title, node_type, parent_id, description, attributes, sort_order, \
                    ARRAY[sort_order] AS path \
             FROM narrative_node \
             WHERE project_id = $1 AND parent_id IS NULL AND status != 'Deleted' \
             UNION ALL \
             SELECT n.id, n.title, n.node_type, n.parent_id, n.description, n.attributes, n.sort_order, \
                    t.path || n.sort_order \
             FROM narrative_node n \
             JOIN tree t ON n.parent_id = t.id \
             WHERE n.project_id = $1 AND n.status != 'Deleted' \
         ) \
         SELECT id, title, node_type, parent_id, description, attributes, path FROM tree ORDER BY path",
    )
    .bind(project_id)
    .fetch_all(pool)
    .await
    .context("按树序读取叙事节点失败")?;

    let title_by_id: HashMap<Uuid, String> =
        rows.iter().map(|r| (r.0, r.1.clone())).collect();
    let count_of = |kind: &str| rows.iter().filter(|r| r.2 == kind).count() as i64;

    let volumes = count_of("Volume");
    let arcs = count_of("Arc");
    let chapters = count_of("Chapter");
    let scenes = count_of("Scene");
    // 字段不全的场景：判定口径与 beats.scenes 步完全一致（同一个 helper）
    let scenes_missing_fields = rows
        .iter()
        .filter(|r| r.2 == "Scene" && !scene_attributes_ready(&r.5))
        .count() as i64;

    let mut hollow_chapters: Vec<Value> = Vec::new();
    let mut chapters_without_scene: Vec<Value> = Vec::new();
    for row in &rows {
        if row.2 != "Chapter" {
            continue;
        }
        let desc_empty = row
            .4
            .as_deref()
            .map(|d| d.trim().is_empty())
            .unwrap_or(true);
        // 注意是「没有**就绪**的场景」：只有 Scene 节点但 attributes 不全，
        // 在正文生成引擎眼里等于没有（它读的就是那几个字段）。
        let has_ready_scene = rows
            .iter()
            .any(|s| s.2 == "Scene" && s.3 == Some(row.0) && scene_attributes_ready(&s.5));
        let entry = json!({
            "id": row.0.to_string(),
            "title": row.1,
            "parent_title": row.3.and_then(|pid| title_by_id.get(&pid).cloned()),
        });
        // 空壳章与「缺场景」是两件事：没有一句话事件的章先补文案，再谈展开场景。
        if desc_empty {
            hollow_chapters.push(entry);
        } else if !has_ready_scene {
            chapters_without_scene.push(entry);
        }
    }

    let arcs_without_chapter: Vec<Value> = rows
        .iter()
        .filter(|r| {
            r.2 == "Arc"
                && !rows
                    .iter()
                    .any(|c| c.2 == "Chapter" && c.3 == Some(r.0))
        })
        .map(|r| json!({ "id": r.0.to_string(), "title": r.1 }))
        .collect();

    let storyline_rows: Vec<(String, String, String)> = sqlx::query_as(
        "SELECT s.id::text, s.name, COALESCE(s.importance, 'Normal') FROM storyline s \
         WHERE s.project_id = $1 AND s.importance IN ('Main', 'Important') \
         AND NOT EXISTS (SELECT 1 FROM narrative_node n WHERE n.storyline_id = s.id AND n.status != 'Deleted') \
         ORDER BY s.created_at",
    )
    .bind(project_id)
    .fetch_all(pool)
    .await
    .context("统计未挂节点的重要故事线失败")?;
    let storylines_without_node: Vec<Value> = storyline_rows
        .iter()
        .map(|(id, name, importance)| json!({ "id": id, "name": name, "importance": importance }))
        .collect();

    let hollow_total = hollow_chapters.len();
    let no_scene_total = chapters_without_scene.len();
    let arc_no_chapter_total = arcs_without_chapter.len();

    // 场景数量门槛取自引导流程定义本身（beats.scenes 步的 min_scenes），
    // 不在这里另写一个数字——两处数字必然分叉，而分叉之后没人知道哪份对。
    let min_scenes_required = agent::guide::STEPS
        .iter()
        .find(|s| s.key == agent::guide::STEP_BEATS_SCENES)
        .and_then(|s| match &s.min_complete {
            agent::guide::MinComplete::BeatsScenes { min_scenes } => Some(*min_scenes),
            _ => None,
        })
        .ok_or_else(|| {
            anyhow::anyhow!(
                "引导流程定义不一致：{} 步未使用 MinComplete::BeatsScenes",
                agent::guide::STEP_BEATS_SCENES
            )
        })?;

    // 下一步建议：按"先粗后细"的顺序判断，命中即返回——AI 拿到它就不用猜该干什么。
    let next_suggestion = if volumes == 0 {
        json!({
            "action": "先建卷",
            "reason": "项目里还没有任何卷节点（node_type='Volume'），细纲的顶层就是卷",
            "tool": "create_node / bulk_create_nodes",
        })
    } else if arc_no_chapter_total > 0 {
        json!({
            "action": "给这些弧补章",
            "reason": format!("有 {} 个弧下面一个章都没有——弧不能是空壳", arc_no_chapter_total),
            "arcs": arcs_without_chapter.iter().take(OUTLINE_LIST_MAX).collect::<Vec<_>>(),
            "tool": "create_node(parent_id=弧 id, node_type='Chapter')",
        })
    } else if hollow_total > 0 {
        json!({
            "action": "给这些章补一句话事件",
            "reason": format!(
                "有 {} 章没有 description（空壳章）；每章都要写清「谁做了什么、结果如何」，这是后面写正文的唯一依据",
                hollow_total
            ),
            "chapters": hollow_chapters.iter().take(OUTLINE_LIST_MAX).collect::<Vec<_>>(),
            "tool": "revise_node（写 description）",
        })
    } else if !storylines_without_node.is_empty() {
        json!({
            "action": "把重要故事线挂到节点上",
            "reason": "这些重要线（Main / Important）还没有任何节点认领它们",
            "storylines": storylines_without_node,
            "tool": "revise_node / create_node（storyline_id + arc_stage）",
        })
    } else if no_scene_total > 0 || scenes_missing_fields > 0 || scenes < min_scenes_required {
        // 三种情况都归到同一步：章下面没有场、场有了但字段不全、场景总数不够。
        // 它们的修法相同（继续展开/补齐场景），所以合成一条建议，但把原因分别说清。
        let mut reasons: Vec<String> = Vec::new();
        if no_scene_total > 0 {
            reasons.push(format!("还有 {} 章没有就绪的场景", no_scene_total));
        }
        if scenes_missing_fields > 0 {
            reasons.push(format!(
                "有 {} 个场景缺少必填属性（{}）",
                scenes_missing_fields,
                agent::guide::SCENE_REQUIRED_FIELDS.join(" / ")
            ));
        }
        if scenes < min_scenes_required {
            reasons.push(format!(
                "场景总数 {} 个，引导流程要求至少 {} 个",
                scenes, min_scenes_required
            ));
        }
        json!({
            "action": "把接下来要写的几章展开成场景",
            "reason": format!(
                "{}；细纲只要保证「前方 3-5 章是细的」，不要全书一次展开",
                reasons.join("；")
            ),
            "chapters": chapters_without_scene.iter().take(5).collect::<Vec<_>>(),
            "scene_required_fields": agent::guide::SCENE_REQUIRED_FIELDS,
            "tool": "create_node(node_type='Scene', parent_id=章 id)，字段填进 attributes",
        })
    } else {
        json!({
            "action": "可以进入正文写作",
            "reason": "卷 / 弧 / 章 / 场景都已就位，每章都有一句话事件、且场景的必填属性齐全",
            "tool": "写作页：选场景 → AI 生成",
        })
    };

    hollow_chapters.truncate(OUTLINE_LIST_MAX);
    chapters_without_scene.truncate(OUTLINE_LIST_MAX);
    let arcs_without_chapter: Vec<Value> =
        arcs_without_chapter.into_iter().take(OUTLINE_LIST_MAX).collect();

    Ok(json!({
        "counts": {
            "volumes": volumes,
            "arcs": arcs,
            "chapters": chapters,
            "scenes": scenes,
            "scenes_missing_required_fields": scenes_missing_fields,
        },
        "scene_min_required": min_scenes_required,
        "scene_required_fields": agent::guide::SCENE_REQUIRED_FIELDS,
        "hollow_chapters_total": hollow_total,
        "hollow_chapters": hollow_chapters,
        "arcs_without_chapter_total": arc_no_chapter_total,
        "arcs_without_chapter": arcs_without_chapter,
        "storylines_without_node_total": storylines_without_node.len(),
        "storylines_without_node": storylines_without_node,
        "chapters_without_scene_total": no_scene_total,
        "chapters_without_scene": chapters_without_scene,
        "next_suggestion": next_suggestion,
    }))
}

/// 组装「引导进度」只读报告：当前阶段 + 每一步的就绪情况 + 未就绪步骤还缺什么。
///
/// **三个消费方共用这一份实现**：`get_project_status` 工具（AI 回答"还差什么"）、
/// `GET /projects/{id}/guide/status` 接口（前端步骤条打对勾，它调的就是这个工具）、
/// 以及 `confirm_step` 的推进校验（同一个快照 + 同一个 `validate_step`）。
/// 三处若各写一套判定，就会出现「界面打了勾、点推进却报缺东西」这种无人能查的分叉。
async fn guide_status(pool: &PgPool, project_id: Uuid) -> Result<Value> {
    let (snapshot, config) = project_status_snapshot(pool, project_id).await?;

    let current_step = config
        .get("current_step")
        .and_then(|v| v.as_str())
        .unwrap_or(agent::guide::INITIAL_STEP)
        .to_string();

    let mut steps = Vec::new();
    for step in agent::guide::STEPS {
        let report = validate_step(step.key, &snapshot);
        let status = match step.key {
            k if k == current_step => "current",
            // 终点步（next 为空，现在的「正文」）的判据是 AlwaysSatisfied ——
            // 它在流程第一步就已经"通过"了。若照常打勾，界面会在故事脑洞阶段
            // 就显示「正文 ✓」，让人误以为全书已经写完。
            _ if step.next.is_empty() => "pending",
            _ if report.passed => "complete",
            _ => "pending",
        };
        steps.push(json!({
            "key": step.key,
            "title": step.title,
            "group": step.group,
            "status": status,
            "ready": report.passed,
            "next": if step.next.is_empty() { Value::Null } else { json!(step.next) },
            "missing": report.missing,
        }));
    }

    let current = steps
        .iter()
        .find(|s| s.get("key").and_then(|v| v.as_str()) == Some(current_step.as_str()))
        .cloned();

    Ok(json!({
        "current_step": current_step,
        "current_title": current
            .as_ref()
            .and_then(|c| c.get("title").cloned())
            .unwrap_or(Value::Null),
        // 当前阶段是否已满足推进条件：true 时应当提示用户可以点「确认推进」
        "current_ready": current
            .as_ref()
            .and_then(|c| c.get("ready").cloned())
            .unwrap_or(json!(false)),
        "steps": steps,
    }))
}

#[async_trait]
impl AgentTool for GuideTool {
    fn name(&self) -> String {
        "confirm_step".to_string()
    }
    fn description(&self) -> String {
        "【仅前端用户点按钮时触发，agent 永远不要主动调】\
         校验当前引导步骤（或指定目标步骤）的最小完整度：满足则把 project.config.current_step \
         推进到目标步骤，不满足则返回结构化错误（包含还差什么）。\n\
         target_step 可选：不传则推进到当前 step 的 next（默认行为）；\
         传入则跳转到指定 step（用于血肉小选择器：用户在血肉 step 之间自由选择下一个）。"
            .into()
    }
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "project_id": { "type": "string" },
                "target_step": { "type": "string", "description": "可选：跳转到指定 step key（血肉之间自由顺序用）" }
            },
            "required": ["project_id"]
        })
    }

    async fn execute(&self, input: Value) -> Result<Value> {
        let project_id = parse_uuid(&input, "project_id")?;

        let (snapshot, config) = project_status_snapshot(&self.pool, project_id).await?;

        // 8. 决定 current_step（优先 config.current_step，否则 premise 状态决定）
        let stored_step = config
            .get("current_step")
            .and_then(|v| v.as_str())
            .unwrap_or(agent::guide::INITIAL_STEP)
            .to_string();

        // 可选 target_step：用户从血肉小选择器点过来时传入；
        // 不传则按 stored_step 校验 + 推进到 next
        let target_step = input
            .get("target_step")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        // 校验目标 step：默认是当前 step；带 target_step 时跳到目标 step 校验
        let step_to_validate = target_step.clone().unwrap_or_else(|| stored_step.clone());

        // 9. 校验
        let mut report: ValidationReport = validate_step(&step_to_validate, &snapshot);

        // 10. 通过则推进
        if report.passed {
            // 推进目标：默认 = report.next_step（用当前 step 算的）；带 target_step = target_step 本身
            let new_step = if target_step.is_some() {
                target_step.as_ref().unwrap().clone()
            } else {
                report.next_step.clone().unwrap_or_default()
            };

            if !new_step.is_empty() {
                // 用 jsonb_set 写入新 step（保留 config 其他键）
                sqlx::query(
                    "UPDATE project SET config = jsonb_set(COALESCE(config, '{}'::jsonb), '{current_step}', to_jsonb($1::text)), updated_at = NOW() WHERE id = $2",
                )
                .bind(&new_step)
                .bind(project_id)
                .execute(&self.pool)
                .await
                .context("Failed to update project.config.current_step")?;
                // 修正 report 的 current_step / next_step 让前端看到正确的状态
                report.current_step = new_step.clone();
                report.next_step = agent::guide::next_step_key(&new_step).map(|s| s.to_string());
                if let Some(next) = &report.next_step {
                    report.next_title = agent::guide::find_step(next).map(|s| s.title.to_string());
                }
            }
        }

        Ok(serde_json::to_value(&report)?)
    }
}


// ============================================================
// 地点档案（location_profile + location_identity 两张表，对外视为一个整体）
// ============================================================

#[derive(Clone, Copy)]
pub enum LocationProfileAction {
    Get,
    Update,
}

pub struct LocationProfileTool {
    action: LocationProfileAction,
    service: Arc<EntityService>,
}

impl LocationProfileTool {
    pub fn new(action: LocationProfileAction, service: Arc<EntityService>) -> Self {
        Self { action, service }
    }
}

/// 注册地点档案工具。
pub fn register_location_profile_tools(registry: &ToolRegistry, service: Arc<EntityService>) {
    for a in [LocationProfileAction::Get, LocationProfileAction::Update] {
        registry.register(Arc::new(LocationProfileTool::new(a, service.clone())));
    }
}

fn location_profile_name(a: LocationProfileAction) -> &'static str {
    match a {
        LocationProfileAction::Get => "get_location_profile",
        LocationProfileAction::Update => "update_location_profile",
    }
}

fn location_profile_description(a: LocationProfileAction) -> String {
    match a {
        LocationProfileAction::Get => {
            "读取某地点的设计档案（地点类型 / 规模 / 气候 / 纪元 / 可达性 / 人口 / 地理 / \
             外貌 / 经济 / 规则 / 历史 / 叙事用途），以及阶段弧线 arc_stages（该地点在故事 \
             不同阶段承担的叙事角色）。更新档案前先读一次，可避免覆盖已有内容。"
                .into()
        }
        LocationProfileAction::Update => {
            "填写或更新地点的设计档案。**只传需要设置的字段**，未传的字段保持原值。\
             创建地点后应当随即补全档案——否则该地点的详情面板会是一片空白，设定等于没落地。\
             若该地点的**叙事角色**随剧情变化（主角藏身处→教团总部→主战场），用 arc_stages \
             按阶段记录 role / function / screen_weight / status；被烧毁、易主这类物理变化属于\
             剧情事件，不要写进阶段。arc_stages 默认整块替换，需要按阶段名局部增删改时传 \
             arc_stages_mode=\"merge\" + remove_arc_stages。"
                .into()
        }
    }
}

fn location_profile_schema(a: LocationProfileAction) -> Value {
    match a {
        LocationProfileAction::Get => json!({
            "type": "object",
            "properties": {
                "id": { "type": "string", "description": "地点实体 UUID" },
                "include_schema": { "type": "boolean", "description": "**可选**：为 true 时额外返回每个可写字段的完整结构（type / enum / items / properties）。默认只返回字段名清单——完整结构约 6000 字符/次，不需要就别要" }
            },
            "required": ["id"]
        }),
        LocationProfileAction::Update => {
            let mut props = json!({
                "id": { "type": "string", "description": "地点实体 UUID" },
                "location_type": { "type": "string", "description": "地点类型，如 城镇/遗迹/秘境" },
                "size": { "type": "string", "description": "规模" },
                "climate": { "type": "string", "description": "气候" },
                "era": { "type": "string", "description": "纪元 / 所属时代" },
                "accessibility": { "type": "string", "description": "可达性（如何抵达、有无限制）" },
                "population": { "type": "string", "description": "人口" },
                "geography": { "type": "string", "description": "地理" },
                "appearance": { "type": "string", "description": "外貌 / 景观" },
                "economy": { "type": "string", "description": "经济" },
                "rules": { "type": "string", "description": "该地特有的规则" },
                "history": { "type": "string", "description": "历史" },
                "narrative_usage": { "type": "string", "description": "叙事用途（在故事里承担什么作用）" },
                "aliases": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "别名 / 别称（整块替换）。用于「盘脊」这类俗称、旧称、异名——不要再把别名塞进 name 的括号里：那样系统里它们会是两个不同的串，按别名检索与去重都做不到。只增删个别别名请用 aliases_add / aliases_remove。"
                },
                "aliases_add": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "只追加这些别名（保留已有别名，重复的自动跳过）"
                },
                "aliases_remove": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "只移除这些别名"
                },
                "secrets": { "type": "string", "description": "地点隐藏的秘密（不为外人所知的事，如「那尊无名神像原本属于谁」）。暗线靠它被按揭示状态管理，不要塞进 description。" }
            });
            props["append"] = json!({
                "type": "boolean",
                "description": "为 true 时把本次传入的文本字段**追加**到原值后面（默认 false = 覆盖）。长文本分次写时用它。"
            });
            // 地点阶段只描述"叙事角色"的变化（主角藏身处→教团总部→主战场）；
            // 被烧毁 / 易主这类物理变化属于事件，不要塞进阶段。
            splice_arc_stage_props(&mut props);
            json!({ "type": "object", "properties": props, "required": ["id"] })
        }
    }
}

#[async_trait]
impl AgentTool for LocationProfileTool {
    fn name(&self) -> String {
        location_profile_name(self.action).to_string()
    }

    fn description(&self) -> String {
        location_profile_description(self.action)
    }

    fn input_schema(&self) -> Value {
        location_profile_schema(self.action)
    }

    async fn execute(&self, input: Value) -> Result<Value> {
        let id = parse_uuid(&input, "id")?;
        match self.action {
            LocationProfileAction::Get => {
                let profile = self.service.get_location_profile(id).await?;
                Ok(json!({
                    "ok": true,
                    "action": "get_location_profile",
                    "data": profile.unwrap_or(json!({})),
                    "writable_fields": writable_fields_for(
                        &input,
                        &location_profile_schema(LocationProfileAction::Update)
                    )
                }))
            }
            LocationProfileAction::Update => {
                // 先读旧档案再合并：upsert 是「整行覆盖」语义，
                // 若直接把本次入参写进去，未传的字段会被清空。
                let existing = self
                    .service
                    .get_location_profile(id)
                    .await?
                    .unwrap_or_else(|| json!({}));

                let mut merged = existing.as_object().cloned().unwrap_or_default();
                if let Some(obj) = input.as_object() {
                    // 字段名写错必须报错：地点档案原先不校验字段名，错名字会写进陌生键、
                    // 落库时被 serde 丢掉，而工具仍回 ok: true（静默丢数据）。
                    agent::ensure_known_fields(
                        &input,
                        &location_profile_schema(LocationProfileAction::Update),
                        &["arc_stages_mode", "remove_arc_stages"],
                    )?;
                    let remove_arc_stages = string_array(obj.get("remove_arc_stages"));
                    let arc_stages_mode = arc_stages_mode_of(obj, &remove_arc_stages);
                    // append=true：文本字段追加写入（长文本分次写，不必重发整段）
                    let append = obj
                        .get("append")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);
                    for (k, v) in obj {
                        if k == "id"
                            || k == "append"
                            || k == "arc_stages_mode"
                            || k == "remove_arc_stages"
                        {
                            continue;
                        }
                        agent::ensure_known_subfields(
                            k,
                            v,
                            &location_profile_schema(LocationProfileAction::Update),
                        )?;
                        // 阶段弧线：人物 / 势力 / 地点同构（归一化 + 按 stage 名称 merge）
                        if k == "arc_stages" {
                            if !v.is_null() {
                                let next = apply_arc_stages(
                                    merged.get(k),
                                    v,
                                    &arc_stages_mode,
                                    &remove_arc_stages,
                                )?;
                                merged.insert(k.clone(), next);
                            }
                            continue;
                        }
                        // 别名：与角色档案同构的合并语义（整块替换 / 增 / 删）
                        if k == "aliases" || k == "aliases_add" || k == "aliases_remove" {
                            let add = string_array(obj.get("aliases_add"));
                            let remove = string_array(obj.get("aliases_remove"));
                            let existing_aliases = merged
                                .get("aliases")
                                .and_then(|v| v.as_array())
                                .cloned()
                                .unwrap_or_default();
                            let list = match obj.get("aliases") {
                                // 传了 aliases：整块替换为它，再叠加 add / remove
                                Some(v) if !v.is_null() => {
                                    let provided: Vec<Value> = v
                                        .as_array()
                                        .cloned()
                                        .ok_or_else(|| anyhow::anyhow!("aliases 应为字符串数组"))?;
                                    merge_aliases(&provided, &add, &remove)
                                }
                                _ => merge_aliases(&existing_aliases, &add, &remove),
                            };
                            merged.insert("aliases".into(), json!(list));
                            continue;
                        }
                        // 其余只接受字符串字段；None / 空串视为"未提供"，保持原值
                        if let Some(s) = v.as_str() {
                            if !s.trim().is_empty() {
                                let value = agent::append_or_replace(
                                    merged.get(k).and_then(|v| v.as_str()),
                                    s,
                                    append,
                                );
                                merged.insert(k.clone(), json!(value));
                            }
                        }
                    }
                    // 只传 remove_arc_stages 而不传 arc_stages 时，也要能删除阶段
                    if arc_stages_mode == "merge" && !remove_arc_stages.is_empty() {
                        let items =
                            remove_arc_stages_only(merged.get("arc_stages"), &remove_arc_stages);
                        merged.insert("arc_stages".into(), items);
                    }
                }

                let saved = self
                    .service
                    .upsert_location_profile(id, Value::Object(merged))
                    .await?;
                // 同样从库里回显名称，方便辨认改的是哪个地点
                let name = require_entity(&self.service, id).await?;
                Ok(json!({
                    "ok": true,
                    "action": "update_location_profile",
                    "name": name,
                    "data": saved
                }))
            }
        }
    }
}


/// 批量填写 / 更新地点档案。
///
/// 逐条调用 `update_location_profile` 时，N 个地点就要 N 次工具调用，
/// 「把没有档案的地点都补上」这种请求很容易撞上单轮调用上限；
/// 批量工具把 N 次合并为一次。
///
/// 单个地点出错**不会中断整批**：返回值里逐个列出失败项与原因，
/// 模型可以据此只重试失败的那几个。
/// 批量地点工具里单个 item 的字段契约。
///
/// 与单条 `update_location_profile` 的字段保持一致（含 `aliases` / `secrets` 与
/// 阶段弧线）——批量补档案时能写的字段，不该比单条少。
fn bulk_location_item_schema() -> Value {
    let mut item_props = json!({
        "id": { "type": "string", "description": "地点实体 UUID（必填）" },
        "location_type": { "type": "string" },
        "size": { "type": "string" },
        "climate": { "type": "string" },
        "era": { "type": "string" },
        "accessibility": { "type": "string" },
        "population": { "type": "string" },
        "geography": { "type": "string" },
        "appearance": { "type": "string" },
        "economy": { "type": "string" },
        "rules": { "type": "string" },
        "history": { "type": "string" },
        "narrative_usage": { "type": "string" },
        "aliases": {
            "type": "array",
            "items": { "type": "string" },
            "description": "别名 / 别称（整块替换）。只增删个别别名请用 aliases_add / aliases_remove"
        },
        "aliases_add": {
            "type": "array",
            "items": { "type": "string" },
            "description": "只追加这些别名（保留已有别名，重复的自动跳过）"
        },
        "aliases_remove": {
            "type": "array",
            "items": { "type": "string" },
            "description": "只移除这些别名"
        },
        "secrets": { "type": "string", "description": "地点隐藏的秘密（不为外人所知的事）。暗线靠它被按揭示状态管理，不要塞进 description。" }
    });
    // 阶段弧线（与单条 update_location_profile 一致）
    splice_arc_stage_props(&mut item_props);
    json!({ "type": "object", "properties": item_props, "required": ["id"] })
}

pub struct BulkLocationProfileTool {
    service: Arc<EntityService>,
}

impl BulkLocationProfileTool {
    pub fn new(service: Arc<EntityService>) -> Self {
        Self { service }
    }

    /// 先读旧档案再合并（与单条工具同一策略）：只覆盖本次提供的字段。
    async fn merge_and_save(
        &self,
        id: Uuid,
        item: &Value,
    ) -> anyhow::Result<Value> {
        let existing = self
            .service
            .get_location_profile(id)
            .await?
            .unwrap_or_else(|| json!({}));
        let mut merged = existing.as_object().cloned().unwrap_or_default();
        if let Some(obj) = item.as_object() {
            // 字段名 / 子字段名与单条 update_location_profile 一样严格：
            // 写错的名字会被 serde 静默丢掉，工具却回 ok: true。
            agent::ensure_known_fields(
                item,
                &bulk_location_item_schema(),
                &["arc_stages_mode", "remove_arc_stages"],
            )?;
            let remove_arc_stages = string_array(obj.get("remove_arc_stages"));
            let arc_stages_mode = arc_stages_mode_of(obj, &remove_arc_stages);
            for (k, v) in obj {
                if k == "id" || k == "arc_stages_mode" || k == "remove_arc_stages" {
                    continue;
                }
                agent::ensure_known_subfields(k, v, &bulk_location_item_schema())?;
                // 别名：与单条 update_location_profile 同构（整块替换 / 增 / 删）
                if k == "aliases" || k == "aliases_add" || k == "aliases_remove" {
                    let add = string_array(obj.get("aliases_add"));
                    let remove = string_array(obj.get("aliases_remove"));
                    let existing_aliases = merged
                        .get("aliases")
                        .and_then(|v| v.as_array())
                        .cloned()
                        .unwrap_or_default();
                    let list = match obj.get("aliases") {
                        Some(v) if !v.is_null() => {
                            let provided: Vec<Value> = v
                                .as_array()
                                .cloned()
                                .ok_or_else(|| anyhow::anyhow!("aliases 应为字符串数组"))?;
                            merge_aliases(&provided, &add, &remove)
                        }
                        _ => merge_aliases(&existing_aliases, &add, &remove),
                    };
                    merged.insert("aliases".into(), json!(list));
                    continue;
                }
                // 阶段弧线：与单条 update_location_profile 同构
                if k == "arc_stages" {
                    if !v.is_null() {
                        let next = apply_arc_stages(
                            merged.get(k),
                            v,
                            &arc_stages_mode,
                            &remove_arc_stages,
                        )?;
                        merged.insert(k.clone(), next);
                    }
                    continue;
                }
                if let Some(s) = v.as_str() {
                    if !s.trim().is_empty() {
                        merged.insert(k.clone(), json!(s));
                    }
                }
            }
            if arc_stages_mode == "merge" && !remove_arc_stages.is_empty() {
                let items = remove_arc_stages_only(merged.get("arc_stages"), &remove_arc_stages);
                merged.insert("arc_stages".into(), items);
            }
        }
        self.service
            .upsert_location_profile(id, Value::Object(merged))
            .await
    }
}

#[async_trait]
impl AgentTool for BulkLocationProfileTool {
    fn name(&self) -> String {
        "bulk_update_location_profiles".to_string()
    }

    fn description(&self) -> String {
        "一次为**多个**地点填写 / 更新设计档案（字段同 update_location_profile）。\
         适合「把所有地点的档案补齐」这类批量场景——逐条调用很快会耗尽单轮调用上限。\
         建议每次 3~5 个（受单次输出长度限制）。只传需要设置的字段，未传的保持原值。"
            .to_string()
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "profiles": {
                    "type": "array",
                    "description": "要更新的地点档案数组，建议每次 3~5 条",
                    "items": bulk_location_item_schema()
                }
            },
            "required": ["profiles"]
        })
    }

    async fn execute(&self, input: Value) -> Result<Value> {
        let arr = input
            .get("profiles")
            .and_then(|v| v.as_array())
            .ok_or_else(|| anyhow::anyhow!("profiles 必须是非空数组"))?;
        if arr.is_empty() {
            anyhow::bail!("profiles 不能为空");
        }

        let mut updated = Vec::new();
        let mut failures = Vec::new();

        for item in arr {
            let id = match parse_uuid(item, "id") {
                Ok(v) => v,
                Err(e) => {
                    failures.push(json!({ "index": updated.len(), "error": e.to_string() }));
                    continue;
                }
            };
            match self.merge_and_save(id, item).await {
                Ok(_) => {
                    /*
                     * 回显的名称必须**从库里查**，不能取入参：
                     * 批量更新时调用方通常只传 id + 要改的字段，入参里根本没有 name，
                     * 取入参只会得到一堆 null，看着像出错。
                     */
                    let name = require_entity(&self.service, id).await?;
                    updated.push(json!({ "id": id.to_string(), "name": name }));
                }
                Err(e) => failures.push(json!({ "id": id.to_string(), "error": e.to_string() })),
            }
        }

        Ok(json!({
            "ok": failures.is_empty(),
            "action": "bulk_update_location_profiles",
            "updated_count": updated.len(),
            "updated": updated,
            "failures": failures
        }))
    }
}

// ============================================================
// 金手指结构化档案工具
// ============================================================

/// 金手指的结构化档案存放在 `entity.attributes`（jsonb）。
///
/// ## 为什么必须结构化，而不是往 description 里写散文
///
/// 金手指是**规则**（怎么运作、代价是什么、边界在哪、现在强到什么程度），
/// 不是普通实体档案。写成一大段散文塞进 `description` 的后果是：
/// 前端只能原样铺出来，用户无法一眼看出"它能干什么、什么时候会失效"；
/// 想做成能力面板就只能靠正则去解析散文，格式一换就崩。
///
/// 结构化的第二个收益是给模型自己用：写章节时读到的是明确字段，
/// 而不是五百字散文里夹着的约束，设定不容易写崩。
///
/// ## 字段语义（明确的，不做隐式猜测）
///
/// - 单值字段 `gf_type` / `one_liner` / `origin` / `side_effect`：
///   非空才覆盖；未传或空串视为"保持原值"。
/// - 列表字段 `abilities` / `constraints` / `growth_stages`：
///   传了就**整体替换**。数组元素没有稳定标识，按索引或按名字合并的语义
///   含糊且结果不可预期，因此不合并——要改就重新给完整列表。
pub struct GoldenFingerProfileTool {
    action: GoldenFingerAction,
    service: Arc<EntityService>,
}

#[derive(Clone, Copy)]
pub enum GoldenFingerAction {
    Get,
    Update,
}

impl GoldenFingerProfileTool {
    pub fn new(action: GoldenFingerAction, service: Arc<EntityService>) -> Self {
        Self { action, service }
    }
}

/// 构造并注册金手指档案工具（读取 + 写入）。
pub fn register_golden_finger_tools(registry: &ToolRegistry, service: Arc<EntityService>) {
    for a in [GoldenFingerAction::Get, GoldenFingerAction::Update] {
        registry.register(Arc::new(GoldenFingerProfileTool::new(a, service.clone())));
    }
}

/// 单值字段：写了非空值才生效。
const GF_SCALAR_FIELDS: [&str; 4] = ["gf_type", "one_liner", "origin", "side_effect"];

/// 列表字段：传了就整体替换。
const GF_LIST_FIELDS: [&str; 3] = ["abilities", "constraints", "growth_stages"];

fn golden_finger_schema(action: GoldenFingerAction) -> Value {
    match action {
        GoldenFingerAction::Get => json!({
            "type": "object",
            "properties": {
                "id": { "type": "string", "description": "金手指实体 UUID" },
                "include_schema": { "type": "boolean", "description": "**可选**：为 true 时额外返回每个可写字段的完整结构（type / enum / items / properties）。默认只返回字段名清单——完整结构约 6000 字符/次，不需要就别要" }
            },
            "required": ["id"]
        }),
        GoldenFingerAction::Update => json!({
            "type": "object",
            "properties": {
                "id": { "type": "string", "description": "金手指实体 UUID" },
                "gf_type": { "type": "string", "description": "金手指类型，如 系统/神格/重生/宝石" },
                "one_liner": { "type": "string", "description": "一句话说清它的作用（不超过 30 字，例如「把信徒的信仰变成神力和金钱」）" },
                "origin": { "type": "string", "description": "来源：怎么得到的、什么来历" },
                "abilities": {
                    "type": "array",
                    "description": "核心机制列表。直接传此字段表示整块替换；只想增删单个机制时改用 abilities_add / abilities_remove。",
                    "items": {
                        "type": "object",
                        "properties": {
                            "name": { "type": "string", "description": "机制名，如 信仰收集" },
                            "effect": { "type": "string", "description": "具体效果：能做什么、数值是多少" },
                            "trigger": { "type": "string", "description": "触发条件：满足什么才会生效" },
                            "limit": { "type": "string", "description": "该机制自身的限制" }
                        },
                        "required": ["name", "effect"]
                    }
                },
                "abilities_add": {
                    "type": "array",
                    "description": "要新增或按 name 覆盖的核心机制，不会清空其它机制",
                    "items": {
                        "type": "object",
                        "properties": {
                            "name": { "type": "string" },
                            "effect": { "type": "string" },
                            "trigger": { "type": "string" },
                            "limit": { "type": "string" }
                        },
                        "required": ["name", "effect"]
                    }
                },
                "abilities_remove": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "要按 name 删除的核心机制名列表"
                },
                "side_effect": { "type": "string", "description": "副作用 / 代价。作者设定为「无」时必须显式写「无」，不要留空。" },
                "constraints": {
                    "type": "array",
                    "description": "硬约束清单。直接传此字段表示整块替换；只想增删单条时改用 constraints_add / constraints_remove。",
                    "items": { "type": "string" }
                },
                "constraints_add": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "要新增的硬约束"
                },
                "constraints_remove": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "要删除的硬约束原文"
                },
                "growth_stages": {
                    "type": "array",
                    "description": "成长阶段。直接传此字段表示整块替换；只想增删单个阶段时改用 growth_stages_add / growth_stages_remove。",
                    "items": {
                        "type": "object",
                        "properties": {
                            "stage": { "type": "string", "description": "阶段名，如 初期/中期/后期" },
                            "unlocked": { "type": "string", "description": "该阶段能做到什么" },
                            "note": { "type": "string", "description": "该阶段的限制或前提" }
                        },
                        "required": ["stage", "unlocked"]
                    }
                },
                "growth_stages_add": {
                    "type": "array",
                    "description": "要新增或按 stage 覆盖的成长阶段",
                    "items": {
                        "type": "object",
                        "properties": {
                            "stage": { "type": "string" },
                            "unlocked": { "type": "string" },
                            "note": { "type": "string" }
                        },
                        "required": ["stage", "unlocked"]
                    }
                },
                "growth_stages_remove": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "要按 stage 删除的成长阶段名列表"
                }
            },
            "required": ["id"]
        }),
    }
}

#[async_trait]
impl AgentTool for GoldenFingerProfileTool {
    fn name(&self) -> String {
        match self.action {
            GoldenFingerAction::Get => "get_golden_finger".to_string(),
            GoldenFingerAction::Update => "update_golden_finger".to_string(),
        }
    }

    fn description(&self) -> String {
        match self.action {
            GoldenFingerAction::Get => {
                "读取金手指的结构化档案（类型 / 一句话作用 / 来源 / 核心机制 / 副作用 / 硬约束 / 成长阶段）。\
                 修改前先读一次，避免覆盖已聊定的内容。"
                    .to_string()
            }
            GoldenFingerAction::Update => {
                "填写 / 更新金手指的结构化档案。**金手指必须结构化**，不能只写在 description 里——\
                 界面的金手指面板完全依赖这些字段，字段为空则面板一片空白。\
                 单值字段只传要改的；abilities / constraints / growth_stages 直接传表示整块替换，\
                 只想增删单条时用 abilities_add / abilities_remove、constraints_add / constraints_remove、\
                 growth_stages_add / growth_stages_remove。\
                 constraints（硬约束）尤其重要：写上「什么时候它会失效」，避免后续章节写崩。"
                    .to_string()
            }
        }
    }

    fn input_schema(&self) -> Value {
        golden_finger_schema(self.action)
    }

    async fn execute(&self, input: Value) -> Result<Value> {
        let id = parse_uuid(&input, "id")?;
        let entity = self
            .service
            .get_entity(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("实体 {} 不存在", id))?;
        let name = entity.get("name").and_then(|v| v.as_str()).map(str::to_string);

        match self.action {
            GoldenFingerAction::Get => Ok(json!({
                "ok": true,
                "action": "get_golden_finger",
                "name": name,
                "data": entity.get("attributes").cloned().unwrap_or(json!({})),
                "writable_fields": writable_fields_for(
                    &input,
                    &golden_finger_schema(GoldenFingerAction::Update)
                )
            })),
            GoldenFingerAction::Update => {
                let mut attrs = entity
                    .get("attributes")
                    .cloned()
                    .unwrap_or_else(|| json!({}));
                let obj = attrs.as_object_mut().ok_or_else(|| {
                    anyhow::anyhow!("实体 {} 的 attributes 不是 JSON 对象，无法写入结构化档案", id)
                })?;

                for k in GF_SCALAR_FIELDS {
                    if let Some(s) = input.get(k).and_then(|v| v.as_str()) {
                        if !s.trim().is_empty() {
                            obj.insert(k.to_string(), json!(s));
                        }
                    }
                }
                for k in GF_LIST_FIELDS {
                    // 直接传列表仍然是整块替换（保持向后兼容）。
                    if let Some(v) = input.get(k).filter(|v| !v.is_null()) {
                        obj.insert(k.to_string(), v.clone());
                        continue;
                    }

                    // 否则支持 *_add / *_remove：只增删单条，避免整块重写。
                    let add_key = format!("{}_add", k);
                    let remove_key = format!("{}_remove", k);
                    let add = input
                        .get(&add_key)
                        .and_then(|v| v.as_array())
                        .cloned()
                        .unwrap_or_default();
                    let remove = input
                        .get(&remove_key)
                        .and_then(|v| v.as_array())
                        .cloned()
                        .unwrap_or_default();

                    if add.is_empty() && remove.is_empty() {
                        continue;
                    }

                    let existing = obj
                        .get(k)
                        .and_then(|v| v.as_array())
                        .cloned()
                        .unwrap_or_default();
                    let merged = if k == "constraints" {
                        merge_string_list(&existing, &add, &remove)
                    } else {
                        let item_key = if k == "abilities" { "name" } else { "stage" };
                        merge_object_list_by_key(&existing, &add, &remove, item_key)
                    };
                    obj.insert(k.to_string(), Value::Array(merged));
                }

                let saved = self
                    .service
                    .update_entity(id, None, None, None, Some(&attrs))
                    .await?;
                Ok(json!({
                    "ok": true,
                    "action": "update_golden_finger",
                    "name": name,
                    "data": saved.get("attributes").cloned().unwrap_or(json!({}))
                }))
            }
        }
    }
}

// ============================================================
// 势力设计档案工具
// ============================================================

/// 势力设计档案（字段与 `faction_profile` 表一一对应）。
///
/// 与地点档案同一策略：**先读旧档案再合并写入**。
/// `upsert_faction_profile` 是整行覆盖语义，若直接把本次入参写进去，
/// 没传的字段会被清空。
pub struct FactionProfileTool {
    action: FactionProfileAction,
    service: Arc<EntityService>,
}

#[derive(Clone, Copy)]
pub enum FactionProfileAction {
    Get,
    Update,
}

impl FactionProfileTool {
    pub fn new(action: FactionProfileAction, service: Arc<EntityService>) -> Self {
        Self { action, service }
    }
}

/// 构造并注册势力设计档案工具（读取 + 写入）。
pub fn register_faction_profile_tools(registry: &ToolRegistry, service: Arc<EntityService>) {
    for a in [FactionProfileAction::Get, FactionProfileAction::Update] {
        registry.register(Arc::new(FactionProfileTool::new(a, service.clone())));
    }
}

/// 势力档案的字段清单（键名即 `faction_profile` 的列名，全部为字符串字段）。
const FACTION_PROFILE_FIELDS: [&str; 11] = [
    "goals",
    "leader",
    "values",
    "resources",
    "territory",
    "members",
    "enemies",
    "allies",
    "internal_conflicts",
    "secrets",
    "modus_operandi",
];

/// 势力档案返回里可能出现、但不可写的系统字段
/// （`upsert_faction_profile` 的返回值会带上 `entity_id`，模型可能原样回传）。
const FACTION_READONLY_FIELDS: [&str; 3] = ["entity_id", "created_at", "updated_at"];

/// 把实体类型名规范成库里存储的英文类型名。
///
/// 库里存的是 `Character` / `Location` / `golden_finger` 这类英文名，
/// 但这是中文小说创作系统——作者和模型都会自然地写「人物」「地点」「金手指」。
/// 认识的别名明确映射，不认识的返回 `None` 由调用方报错；
/// 这样既不会因为"写法和库里不一样"就拒绝合理输入，也不会瞎猜一个类型存进去。
fn normalize_entity_type(raw: &str) -> Option<&'static str> {
    let t = raw.trim();
    let lower = t.to_ascii_lowercase();
    // **从 [`ENTITY_TYPES`] 派生**，不再手写第二张 match 表：
    // 以前这里是两份真相，新增类型时漏改一处就会出现「能创建但查不到」或
    // 「列得出但创建不了」的割裂（加 Deity 时就踩到了）。
    for (canonical, aliases) in ENTITY_TYPES {
        if canonical.to_ascii_lowercase() == lower {
            return Some(*canonical);
        }
        if aliases
            .iter()
            .any(|a| *a == t || a.to_ascii_lowercase() == lower)
        {
            return Some(*canonical);
        }
    }
    None
}

/// 合法的实体类型清单：`(规范类型名, 可接受的别名)`。
///
/// 与 [`normalize_entity_type`] 的分工是明确的：那张映射表负责「解析用户写法」，
/// 这张表负责「告诉调用方有哪些类型可写」。两者必须一起改——新增类型时两边都要加，
/// 否则会出现「能创建但查不到」或「列得出但创建不了」的割裂。
const ENTITY_TYPES: &[(&str, &[&str])] = &[
    ("Character", &["人物", "角色"]),
    // 神明 / 神祇：故事的核心存在也需要能建实体、挂关系、进图谱。
    // 此前只有 Character / Creature 等，「神明 vs 对头」只能停在事件描述里当散文。
    ("Deity", &["神明", "神祇", "神灵", "神"]),
    ("Location", &["地点", "场景", "地图"]),
    ("Faction", &["势力", "门派", "宗门"]),
    ("Item", &["物品", "道具", "装备", "法宝", "灵器"]),
    ("golden_finger", &["金手指"]),
    ("Event", &["事件"]),
    ("Creature", &["生物", "怪物", "魔兽"]),
    ("Organization", &["组织", "团体"]),
];

/// 每种类型的一句话说明，让调用方不必靠类型名猜用途。
fn entity_type_note(entity_type: &str) -> &'static str {
    match entity_type {
        "Character" => "人物角色",
        "Deity" => "神明 / 神祇（超越性存在）",
        "Location" => "地点 / 场景",
        "Faction" => "势力 / 门派 / 宗门",
        "Item" => "物品 / 道具 / 装备 / 法宝",
        "golden_finger" => "金手指（主角独有能力体系）",
        "Event" => "历史或剧情事件",
        "Creature" => "生物 / 怪物 / 魔兽",
        "Organization" => "组织 / 团体（非门派类）",
        _ => "",
    }
}

/// 唯一的实体类型来源：`list_entity_types` 的返回值。
///
/// 存在的理由：类型名原先只硬编码在 Rust 的 `match` 分支里，模型看不到，
/// 只能靠从返回数据里反推（「Character / Location 是推出来的，Faction / Item
/// 是猜的」）。暴露出来之后，检索与创建都不必再猜。
pub fn entity_types_catalog() -> Value {
    Value::Array(
        ENTITY_TYPES
            .iter()
            .map(|(canonical, aliases)| {
                json!({
                    "entity_type": canonical,
                    "aliases": aliases,
                    "note": entity_type_note(canonical),
                })
            })
            .collect(),
    )
}

/// 实体类型无法识别时的统一提示。
const ENTITY_TYPE_HINT: &str = "可传 Character / Location / Faction / Item / Organization / Creature / Event / Deity / golden_finger，\
                                 或直接写中文「人物」「地点」「势力」「物品」「组织」「生物」「事件」「神明」「金手指」";

fn parse_entity_type(raw: &str) -> Result<&'static str> {
    normalize_entity_type(raw)
        .ok_or_else(|| anyhow::anyhow!("未知的实体类型：{}（{}）", raw, ENTITY_TYPE_HINT))
}

/// 校验实体存在，并返回它的名称（不存在则报错）。
///
/// 模型有时会拿着**过期或凭空构造的 UUID** 来写档案（实测出现过：id 在库里
/// 根本不存在）。若不在这里拦住，最终报出来的是数据库外键约束错误
/// （`violates foreign key constraint "..."`），模型既读不懂，也不知道
/// 应该先去查一遍真实 id，于是只能反复重试。
async fn require_entity(service: &Arc<EntityService>, id: Uuid) -> Result<Option<String>> {
    match service.get_entity(id).await? {
        Some(e) => Ok(e
            .get("name")
            .and_then(|v| v.as_str())
            .map(str::to_string)),
        None => anyhow::bail!(
            "实体 {} 不存在。请先用 list_entities 查到正确的 id 再操作，不要凭记忆构造 UUID。",
            id
        ),
    }
}

fn faction_profile_field_spec() -> Value {
    json!({
        "goals": { "type": "string", "description": "势力的目标 / 想要什么" },
        "leader": { "type": "string", "description": "领袖是谁" },
        "values": { "type": "string", "description": "价值观 / 行事准则" },
        "resources": { "type": "string", "description": "掌握的资源（财力、法宝、人手）" },
        "territory": { "type": "string", "description": "领地 / 势力范围" },
        "members": { "type": "string", "description": "成员构成（层级、规模、主力）" },
        "enemies": { "type": "string", "description": "敌人 / 敌对势力" },
        "allies": { "type": "string", "description": "盟友 / 合作方" },
        "internal_conflicts": { "type": "string", "description": "内部矛盾（派系斗争、隐患）" },
        "secrets": { "type": "string", "description": "秘密（不为外人所知的事）" },
        "modus_operandi": { "type": "string", "description": "行事风格 / 惯用手段" }
    })
}

fn faction_profile_update_schema() -> Value {
    let mut props = faction_profile_field_spec();
    props["id"] = json!({ "type": "string", "description": "势力实体 UUID" });
    props["append"] = json!({
        "type": "boolean",
        "description": "为 true 时把本次传入的文本字段**追加**到原值后面（默认 false = 覆盖）。长文本分次写时用它。"
    });
    props["aliases"] = json!({
        "type": "array",
        "items": { "type": "string" },
        "description": "别名 / 旧称 / 俗称（整块替换）。用于「炼器神宗」的旧号、简称等——不要塞进 name 的括号里，那样系统里会是两个不同的串。只增删个别别名请用 aliases_add / aliases_remove。"
    });
    props["aliases_add"] = json!({
        "type": "array",
        "items": { "type": "string" },
        "description": "只追加这些别名（保留已有别名，重复的自动跳过）"
    });
    props["aliases_remove"] = json!({
        "type": "array",
        "items": { "type": "string" },
        "description": "只移除这些别名"
    });
    // 势力最需要阶段：崛起→扩张→鼎盛→分裂；status 用来记录"这一卷多强"
    splice_arc_stage_props(&mut props);
    json!({ "type": "object", "properties": props, "required": ["id"] })
}

#[async_trait]
impl AgentTool for FactionProfileTool {
    fn name(&self) -> String {
        match self.action {
            FactionProfileAction::Get => "get_faction_profile".to_string(),
            FactionProfileAction::Update => "update_faction_profile".to_string(),
        }
    }

    fn description(&self) -> String {
        match self.action {
            FactionProfileAction::Get => {
                "读取某个势力的设计档案（目标 / 领袖 / 价值观 / 资源 / 领地 / 成员 / 敌人 / \
                 盟友 / 内部矛盾 / 秘密 / 行事风格），以及阶段弧线 arc_stages（崛起→鼎盛→分裂）。\
                 更新前先读一次，可避免覆盖已有内容。"
                    .to_string()
            }
            FactionProfileAction::Update => {
                "填写 / 更新势力的设计档案。**只传需要设置的字段**，未传的字段保持原值。\
                 建完势力后应当随即补全档案——否则势力详情面板会是一片空白，设定等于没落地。\
                 势力的强盛/地盘/盟友会随剧情变化，用 arc_stages 按阶段记录：\
                 role（在故事里的角色）/ goal（该阶段目标）/ status（该阶段多强、占哪、跟谁结盟）；\
                 arc_stages 默认整块替换，想按阶段名局部增删改就传 arc_stages_mode=\"merge\" + remove_arc_stages。"
                    .to_string()
            }
        }
    }

    fn input_schema(&self) -> Value {
        match self.action {
            FactionProfileAction::Get => json!({
                "type": "object",
                "properties": {
                    "id": { "type": "string", "description": "势力实体 UUID" },
                "include_schema": { "type": "boolean", "description": "**可选**：为 true 时额外返回每个可写字段的完整结构（type / enum / items / properties）。默认只返回字段名清单——完整结构约 6000 字符/次，不需要就别要" }
                },
                "required": ["id"]
            }),
            FactionProfileAction::Update => faction_profile_update_schema(),
        }
    }

    async fn execute(&self, input: Value) -> Result<Value> {
        let id = parse_uuid(&input, "id")?;
        match self.action {
            FactionProfileAction::Get => {
                let profile = self.service.get_faction_profile(id).await?;
                Ok(json!({
                    "ok": true,
                    "action": "get_faction_profile",
                    "data": profile.unwrap_or(json!({})),
                    "writable_fields": writable_fields_for(&input, &faction_profile_update_schema())
                }))
            }
            FactionProfileAction::Update => {
                // 先读旧档案再合并：upsert 是整行覆盖语义，
                // 若直接把本次入参写进去，未传的字段会被清空。
                let existing = self
                    .service
                    .get_faction_profile(id)
                    .await?
                    .unwrap_or_else(|| json!({}));

                let mut merged = existing.as_object().cloned().unwrap_or_default();
                // 回显名称必须从库里查，入参里通常没有 name
                let name = require_entity(&self.service, id).await?;

                if let Some(obj) = input.as_object() {
                    let remove_arc_stages = string_array(obj.get("remove_arc_stages"));
                    let arc_stages_mode = arc_stages_mode_of(obj, &remove_arc_stages);
                    // append=true：文本字段追加写入（长文本分次写，不必重发整段）
                    let append = obj
                        .get("append")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);
                    for (k, v) in obj {
                        if k == "id"
                            || k == "append"
                            || FRAMEWORK_INJECTED_FIELDS.contains(&k.as_str())
                            || FACTION_READONLY_FIELDS.contains(&k.as_str())
                            || k == "arc_stages_mode"
                            || k == "remove_arc_stages"
                        {
                            continue;
                        }
                        agent::ensure_known_subfields(
                            k,
                            v,
                            &faction_profile_update_schema(),
                        )?;
                        // 阶段弧线：人物 / 势力 / 地点同构（归一化 + 按 stage 名称 merge）
                        if k == "arc_stages" {
                            if !v.is_null() {
                                let next = apply_arc_stages(
                                    merged.get(k),
                                    v,
                                    &arc_stages_mode,
                                    &remove_arc_stages,
                                )?;
                                merged.insert(k.clone(), next);
                            }
                            continue;
                        }
                        // 别名：与角色 / 地点档案同构的合并语义（整块替换 / 增 / 删）
                        if k == "aliases" || k == "aliases_add" || k == "aliases_remove" {
                            let add = string_array(obj.get("aliases_add"));
                            let remove = string_array(obj.get("aliases_remove"));
                            let existing_aliases = merged
                                .get("aliases")
                                .and_then(|v| v.as_array())
                                .cloned()
                                .unwrap_or_default();
                            let list = match obj.get("aliases") {
                                Some(v) if !v.is_null() => {
                                    let provided: Vec<Value> = v
                                        .as_array()
                                        .cloned()
                                        .ok_or_else(|| anyhow::anyhow!("aliases 应为字符串数组"))?;
                                    merge_aliases(&provided, &add, &remove)
                                }
                                _ => merge_aliases(&existing_aliases, &add, &remove),
                            };
                            merged.insert("aliases".into(), json!(list));
                            continue;
                        }
                        if !FACTION_PROFILE_FIELDS.contains(&k.as_str()) {
                            anyhow::bail!(
                                "未知的势力档案字段：{}。可写字段只有：{} / arc_stages",
                                k,
                                FACTION_PROFILE_FIELDS.join(" / ")
                            );
                        }
                        // 只接受非空字符串；空串视为"未提供"，保持原值
                        if let Some(s) = v.as_str() {
                            if !s.trim().is_empty() {
                                let value = agent::append_or_replace(
                                    merged.get(k).and_then(|v| v.as_str()),
                                    s,
                                    append,
                                );
                                merged.insert(k.clone(), json!(value));
                            }
                        }
                    }
                    // 只传 remove_arc_stages 而不传 arc_stages 时，也要能删除阶段
                    if arc_stages_mode == "merge" && !remove_arc_stages.is_empty() {
                        let items =
                            remove_arc_stages_only(merged.get("arc_stages"), &remove_arc_stages);
                        merged.insert("arc_stages".into(), items);
                    }
                }

                let saved = self
                    .service
                    .upsert_faction_profile(id, Value::Object(merged))
                    .await?;
                Ok(json!({
                    "ok": true,
                    "action": "update_faction_profile",
                    "name": name,
                    "data": saved
                }))
            }
        }
    }
}

// ============================================================
// 人物档案 / 状态工具
// ============================================================

/// 人物设计档案（`character_profile`）。
///
/// ## 为什么必须有这个工具
///
/// 人物的档案此前**没有任何写入工具**：AI 只能创建 Character 实体（name + summary +
/// description），前端那张「角色设定」表因此永远是空的。
///
/// ## 枚举字段必须传规范值
///
/// `age_range` / `gender` / `role_in_story` 在后端是枚举，只认下列取值
/// （中文如「男」「青年」一律非法）。传错会**直接报错并列出合法值**，
/// 不会静默降级——降级会让「填了男」变成 `Other` 这种错误数据静悄悄地写进库里。
///
/// ## 合并语义
///
/// 底层是整行覆盖写入，所以这里先读旧档案再合并：
/// 单值字段非空才覆盖；`aliases` 传了就整体替换；
/// `social_position_rank` 只改社会地位的 rank，保留同一对象里的其它子字段。
pub struct CharacterProfileTool {
    action: CharacterProfileAction,
    service: Arc<EntityService>,
}

#[derive(Clone, Copy)]
pub enum CharacterProfileAction {
    Get,
    Update,
}

impl CharacterProfileTool {
    pub fn new(action: CharacterProfileAction, service: Arc<EntityService>) -> Self {
        Self { action, service }
    }
}

/// 人物状态（`character_state`）：角色在故事当下所处的处境。
pub struct CharacterStateTool {
    action: CharacterStateAction,
    service: Arc<EntityService>,
}

#[derive(Clone, Copy)]
pub enum CharacterStateAction {
    Get,
    Update,
}

impl CharacterStateTool {
    pub fn new(action: CharacterStateAction, service: Arc<EntityService>) -> Self {
        Self { action, service }
    }
}

/// 构造并注册人物档案 / 状态工具。
pub fn register_character_tools(registry: &ToolRegistry, service: Arc<EntityService>) {
    for a in [CharacterProfileAction::Get, CharacterProfileAction::Update] {
        registry.register(Arc::new(CharacterProfileTool::new(a, service.clone())));
    }
    for a in [CharacterStateAction::Get, CharacterStateAction::Update] {
        registry.register(Arc::new(CharacterStateTool::new(a, service.clone())));
    }
}

/// 人物档案中的纯文本字段。
const CHAR_TEXT_FIELDS: [&str; 6] = [
    "name",
    "identity",
    "appearance",
    "background_origin",
    "core_personality",
    "values",
];

/// 人物档案里**可写**的字段全集（`update_character_profile` 只认这些）。
const CHAR_WRITABLE_FIELDS: [&str; 18] = [
    "name",
    "aliases",
    "age_range",
    "gender",
    "identity",
    "appearance",
    "background_origin",
    "core_personality",
    "values",
    "role_in_story",
    "social_position",
    "social_position_rank",
    "drive",
    "capabilities",
    "arc_potential",
    "arc_stages",
    "conflicts",
    "secrets",
];

/// 结构化的扩展字段：原样透传给 db 层，由它校验结构并写入对应的扩展表。
///
/// - `drive` / `capabilities` / `arc_potential`：对象
/// - `conflicts` / `secrets`：数组，元素可以是字符串（只给描述/内容），
///   也可以是对象（`{conflict_type, description}` / `{content, importance}`）
/// - `arc_stages`：数组，元素可以是字符串（只给阶段名），也可以是对象
///
/// 注意：`arc_stages` 虽然也是数组，但它有独立的 merge 逻辑（按 `stage` 名称
/// 合并，而不是按 id），所以在 update 分支里单独处理。
const CHAR_STRUCT_FIELDS: [&str; 5] = [
    "drive",
    "capabilities",
    "arc_potential",
    "conflicts",
    "secrets",
];

/// 控制字段：只影响本次更新方式，不写入档案内容。
const CHAR_CONTROL_FIELDS: [&str; 9] = [
    "conflicts_mode",
    "remove_conflict_ids",
    "secrets_mode",
    "remove_secret_ids",
    "arc_stages_mode",
    "remove_arc_stages",
    "aliases_add",
    "aliases_remove",
    "social_position_rank",
];

/// `get_character_profile` 会返回、但 `update_character_profile` **不接受**的字段：
/// 系统列与尚未开放写入的扩展内容。
///
/// 为什么要单独列出来：模型的自然行为是「先 get 读现状，改几个字段，再把整份写回」。
/// 这些字段因此必然会出现在 update 的入参里。若把它们当成"字段名拼错"而报错，
/// 模型会陷入反复重试（实测就会这样），所以这里明确忽略并回执给模型，
/// 而不是抛错——但也绝不静默：被忽略的字段会出现在返回值的 `ignored` 里。
const CHARACTER_READONLY_FIELDS: [&str; 7] = [
    "id",
    "entity_id",
    "created_at",
    "updated_at",
    "relationships",
    "extension",
    "narrative_necessity",
];

/// 人物状态里可写的字段全集。
const CHAR_STATE_WRITABLE_FIELDS: [&str; 6] = [
    "location",
    "physical_state",
    "mental_state",
    "resource_state",
    "social_state",
    "flags",
];

/// 人物状态里只读的字段（含 `extra`：它是历史遗留字段，本工具不负责写）。
const CHARACTER_STATE_READONLY_FIELDS: [&str; 5] =
    ["id", "entity_id", "created_at", "updated_at", "extra"];

/// 人物状态中的纯文本字段。
const CHAR_STATE_TEXT_FIELDS: [&str; 5] = [
    "location",
    "physical_state",
    "mental_state",
    "resource_state",
    "social_state",
];

fn enum_schema(values: &[&str], desc: &str) -> Value {
    json!({ "type": "string", "enum": values, "description": desc })
}

fn character_profile_props() -> Value {
    use domain::character::{AgeRange, Gender, StoryRole};
    let mut props = json!({
        "name": { "type": "string", "description": "真名（可与实体名不同）" },
        "aliases": {
            "type": "array",
            "items": { "type": "string" },
            "description": "别名 / 绰号列表（传了就整体替换）"
        },
        "age_range": enum_schema(&AgeRange::ALL, "年龄段。可直接传中文：儿童 / 少年 / 青年 / 成年 / 中年 / 老年 / 不明"),
        "gender": enum_schema(&Gender::ALL, "性别。可直接传中文：男 / 女 / 非二元 / 不明 / 其他"),
        "identity": { "type": "string", "description": "身份 / 职业" },
        "appearance": { "type": "string", "description": "外貌" },
        "background_origin": { "type": "string", "description": "背景来历" },
        "core_personality": { "type": "string", "description": "核心性格" },
        "values": { "type": "string", "description": "价值观" },
        "role_in_story": enum_schema(&StoryRole::ALL, "在故事中的功能位。可直接传中文：主角 / 反派 / 导师 / 盟友 / 对手 / 催化剂 / 受害者 / 旁观者"),
        "social_position_rank": {
            "type": "string",
            "description": "社会地位 / 头衔（只写这一项，不影响社会地位的其它子字段）"
        },
        "social_position": {
            "type": "object",
            "description": "社会地位（对象形式，整体替换）。读取档案时 data.social_position 就是这个形状；只想改头衔用 social_position_rank",
            "properties": {
                "rank": { "type": "string", "description": "头衔 / 地位" },
                "authority_level": { "type": "integer", "description": "权力层级" },
                "social_access": { "type": "array", "items": { "type": "string" }, "description": "能进入的圈子 / 场所" }
            }
        },
        "drive": {
            "type": ["string", "object"],
            "description": "驱动力：他为什么行动。可直接写一段字符串（会记为 motivation），也可写成对象，用 motivation（核心动机）/ fear（恐惧）/ weakness（弱点）/ desire（欲望）/ contradiction（内在矛盾）/ primary_goal（首要目标）等字段分别描述",
            "properties": {
                "primary_goal": { "type": "string" },
                "motivation": { "type": "string", "description": "核心动机" },
                "urgency": { "type": "integer", "description": "紧迫度 1-10；可选，不传默认 3" },
                "long_term": { "type": "string" },
                "current": { "type": "string" },
                "immediate": { "type": "string" },
                "hidden_goal": { "type": "string" },
                "fear": { "type": "string" },
                "weakness": { "type": "string" },
                "desire": { "type": "string" },
                "contradiction": { "type": "string" },
                "mode": {
                    "type": "string",
                    "enum": ["merge", "replace"],
                    "description": "更新模式：默认 merge（只覆盖传入子字段），replace 表示整体替换"
                }
            }
        },
        "capabilities": {
            "type": "object",
            "description": "能力边界——限制比能力更重要",
            "properties": {
                "skills": { "type": "array", "items": { "type": "string" }, "description": "擅长的能力" },
                "limitations": { "type": "array", "items": { "type": "string" }, "description": "做不到 / 有代价的地方" },
                "mode": {
                    "type": "string",
                    "enum": ["merge", "replace"],
                    "description": "更新模式：默认 merge（只覆盖传入子字段），replace 表示整体替换"
                }
            }
        },
        "arc_potential": {
            "type": "object",
            "description": "弧光潜力：这条人物线要怎么变",
            "properties": {
                "starting_state": { "type": "string", "description": "起点状态" },
                "possible_change": { "type": "string", "description": "可能的变化方向" },
                "resistance": { "type": "string", "description": "阻力来自哪里" },
                "mode": {
                    "type": "string",
                    "enum": ["merge", "replace"],
                    "description": "更新模式：默认 merge（只覆盖传入子字段），replace 表示整体替换"
                }
            }
        },
        "conflicts": {
            "type": "array",
            "description": "冲突：人物真正参与剧情的矛盾。元素可写字符串描述，也可写成对象 {id?, conflict_type?, description, resolution_status?, phase?}。默认整块替换；conflicts_mode=merge 时按 id 更新/新增，未提及的旧冲突保留",
            "items": {
                "type": ["string", "object"],
                "properties": {
                    "id": { "type": "string", "description": "已有冲突 id；不传视为新增" },
                    "conflict_type": { "type": "string", "description": "内在 / 外在 / 关系 / 理念，或 Internal / External / Relationship / Ideology" },
                    "description": { "type": "string" },
                    "resolution_status": { "type": "string" },
                    "phase": { "type": "string", "description": "这条冲突从哪个阶段开始成立，对应 arc_stages[].stage（如「中期」）。不填表示全书一直成立" }
                }
            }
        },
        "conflicts_mode": {
            "type": "string",
            "enum": ["replace", "merge"],
            "description": "conflicts 更新模式：默认 replace（整块替换）；merge 表示按 id 局部更新/新增、未提及的保留"
        },
        "remove_conflict_ids": {
            "type": "array",
            "items": { "type": "string" },
            "description": "conflicts_mode=merge 时要删除的冲突 id 列表"
        },
        "secrets": {
            "type": "array",
            "description": "秘密：驱动悬念与反转的信息差。元素可写字符串，也可写成对象 {id?, content, importance?, reveal_condition?}。默认整块替换；secrets_mode=merge 时按 id 更新/新增，未提及的旧秘密保留",
            "items": {
                "type": ["string", "object"],
                "properties": {
                    "id": { "type": "string", "description": "已有秘密 id；不传视为新增" },
                    "content": { "type": "string" },
                    "importance": { "type": "integer", "description": "1-5，默认 3" },
                    "reveal_condition": { "type": "string" }
                }
            }
        },
        "secrets_mode": {
            "type": "string",
            "enum": ["replace", "merge"],
            "description": "secrets 更新模式：默认 replace（整块替换）；merge 表示按 id 局部更新/新增、未提及的保留"
        },
        "remove_secret_ids": {
            "type": "array",
            "items": { "type": "string" },
            "description": "secrets_mode=merge 时要删除的秘密 id 列表"
        },
        "aliases_add": {
            "type": "array",
            "items": { "type": "string" },
            "description": "要新增的别名（不会清空已有别名）"
        },
        "aliases_remove": {
            "type": "array",
            "items": { "type": "string" },
            "description": "要删除的别名"
        }
    });
    splice_arc_stage_props(&mut props);
    props
}

/// 阶段弧线（arc_stages）的字段定义 —— 人物 / 势力 / 地点共用。
///
/// 通用字段：何时上场（stage/order）、演什么（role/function）、戏份多大（screen_weight）、
/// 该阶段目标（goal）、进入触发（entry_trigger）、现状快照（status）。
/// `status` 对势力尤其重要：势力没有 state 表，只能靠它表达「这一卷多强」。
fn arc_stage_props() -> serde_json::Map<String, Value> {
    use domain::arc_stage::ScreenWeight;
    let obj = json!({
        "arc_stages": {
            "type": "array",
            "description": "阶段弧线（外部时间线）：这个实体在故事不同阶段的身份 / 戏份 / 目标 / 现状。人物（前中后期身份变化）、势力（崛起→鼎盛→分裂）、地点（藏身处→据点→主战场）共用。元素可写字符串（只给阶段名），也可写成对象。默认整块替换；arc_stages_mode=merge 时按 stage 名称合并、未提及的保留",
            "items": {
                "type": ["string", "object"],
                "properties": {
                    "stage": { "type": "string", "description": "阶段名：前期 / 中期 / 后期，或卷1 / 卷2，或按地点命名" },
                    "order": { "type": "integer", "description": "排序用整数，越小越早。不填按数组顺序" },
                    "role": { "type": "string", "description": "此阶段的**定位**（人物 / 势力 / 地点三个语境通用）：人物=这一阶段他是谁（第一个合伙人 / 弃子）；势力=这一阶段它在主角眼里的分量（靠山 / 主要对手 / 背景板）；地点=这一阶段的叙事位置（藏身处 / 据点 / 主战场）。等价写法 stage_role，只传一个即可" },
                    "stage_role": { "type": "string", "description": "role 的等价写法（更中性的叫法，写势力 / 地点时读起来更顺）。与 role 同时给出且不一致会报错" },
                    "screen_weight": enum_schema(&ScreenWeight::ALL, "戏份：Light 轻 / Medium 中 / Heavy 重。不填表示不确定"),
                    "goal": { "type": "string", "description": "此阶段的目标（势力尤其需要：这一卷它想要什么）" },
                    "function": { "type": "string", "description": "此阶段的叙事功能（它为什么在这个阶段存在）" },
                    "entry_trigger": { "type": "string", "description": "什么事件把它推进这一阶段（让阶段变成可推演的）" },
                    "status": { "type": "string", "description": "该阶段的现状快照：势力写「三百人、占三座城、与主角结盟」，地点写「被烧毁一半，流民占据」" }
                },
                "required": ["stage"]
            }
        },
        "arc_stages_mode": {
            "type": "string",
            "enum": ["replace", "merge"],
            "description": "arc_stages 更新模式：默认 replace（整块替换）；merge 表示按 stage 名称局部更新/新增、未提及的保留"
        },
        "remove_arc_stages": {
            "type": "array",
            "items": { "type": "string" },
            "description": "arc_stages_mode=merge 时要删除的阶段名列表（按 stage 匹配）"
        }
    });
    obj.as_object().cloned().unwrap_or_default()
}

/// 把 arc_stages / arc_stages_mode / remove_arc_stages 并入某个 schema 的 properties。
fn splice_arc_stage_props(props: &mut Value) {
    if let Value::Object(map) = props {
        for (k, v) in arc_stage_props() {
            map.insert(k, v);
        }
    }
}

/// 从入参读出 arc_stages_mode：显式给就用；只给了 remove_arc_stages 时默认 merge；
/// 否则默认 replace（整块替换）。
fn arc_stages_mode_of(
    obj: &serde_json::Map<String, Value>,
    remove_stage_names: &[String],
) -> String {
    obj.get("arc_stages_mode")
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_ascii_lowercase())
        .unwrap_or_else(|| {
            if remove_stage_names.is_empty() {
                "replace".to_string()
            } else {
                "merge".to_string()
            }
        })
}

/// 处理 arc_stages 入参：归一化元素，按 mode 整块替换 / 按 stage 名称 merge。
fn apply_arc_stages(
    existing: Option<&Value>,
    incoming: &Value,
    mode: &str,
    remove_stage_names: &[String],
) -> Result<Value> {
    let arr = incoming
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("arc_stages 应为数组"))?;
    let normalized: Vec<Value> = arr
        .iter()
        .map(normalize_arc_stage_item)
        .collect::<Result<Vec<_>>>()?;
    if mode == "merge" {
        let existing_items = existing
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        let remove_values: Vec<Value> = remove_stage_names.iter().map(|s| json!(s)).collect();
        Ok(Value::Array(merge_object_list_by_key(
            &existing_items,
            &normalized,
            &remove_values,
            "stage",
        )))
    } else {
        Ok(Value::Array(normalized))
    }
}

/// 只传 remove_arc_stages 而不传 arc_stages 时的删除。
fn remove_arc_stages_only(existing: Option<&Value>, remove_stage_names: &[String]) -> Value {
    let existing_items = existing
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let remove_values: Vec<Value> = remove_stage_names.iter().map(|s| json!(s)).collect();
    Value::Array(merge_object_list_by_key(
        &existing_items,
        &[],
        &remove_values,
        "stage",
    ))
}

/// 归一化 `drive`：模型有时只传一段动机描述。
///
/// 字符串统一包装成 `{ "motivation": "..." }`，后续再按 merge/replace 处理。
fn normalize_drive(v: &Value) -> Result<Value> {
    if let Some(text) = v.as_str() {
        let text = text.trim();
        if text.is_empty() {
            return Ok(json!({}));
        }
        return Ok(json!({ "motivation": text }));
    }
    if v.is_object() {
        return Ok(v.clone());
    }
    anyhow::bail!("drive 应为对象（motivation / fear / weakness 等），或一段动机描述字符串");
}

/// 拆出结构对象里的 `mode` 控制字段；mode 不落库，只决定 merge/replace。
///
/// 默认 `merge`：未传的子字段保持原值。
/// `replace`：整块替换，未传的子字段清空/回默认值。
fn split_struct_mode(v: &Value) -> (String, Value) {
    if let Some(obj) = v.as_object() {
        let mut payload = obj.clone();
        let mode = payload
            .remove("mode")
            .and_then(|m| m.as_str().map(|s| s.trim().to_string()))
            .filter(|m| !m.is_empty())
            .unwrap_or_else(|| "merge".to_string())
            .to_ascii_lowercase();
        (mode, Value::Object(payload))
    } else {
        ("merge".to_string(), v.clone())
    }
}

/// 浅合并两个 JSON 对象：incoming 覆盖 existing 的同名子字段，existing 独有的保留。
fn merge_object_values(existing: Option<&Value>, incoming: &Value) -> Value {
    let mut merged = existing
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();
    if let Some(obj) = incoming.as_object() {
        for (k, v) in obj {
            merged.insert(k.clone(), v.clone());
        }
    }
    Value::Object(merged)
}

fn string_array(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str())
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        })
        .unwrap_or_default()
}

fn normalize_conflict_item(raw: &Value) -> Value {
    match raw {
        Value::String(s) => json!({ "description": s.trim() }),
        other => other.clone(),
    }
}

fn normalize_secret_item(raw: &Value) -> Value {
    match raw {
        Value::String(s) => json!({ "content": s.trim() }),
        other => other.clone(),
    }
}

/// 按 id 合并结构数组：
/// - remove_ids 中的旧条目删除；
/// - incoming 中带 id 的条目与旧条目浅合并（保留 id 和未传子字段）；
/// - incoming 中没带 id 的条目视为新增；
/// - 旧列表里未提及的条目保留。
fn merge_structured_items(
    kind: &str,
    existing: &[Value],
    incoming: &[Value],
    remove_ids: &[String],
) -> Result<Vec<Value>> {
    let normalize = |raw: &Value| match kind {
        "conflicts" => normalize_conflict_item(raw),
        "secrets" => normalize_secret_item(raw),
        _ => raw.clone(),
    };

    let mut out: Vec<Value> = existing
        .iter()
        .filter(|item| {
            let id = item.get("id").and_then(|v| v.as_str()).unwrap_or("");
            !remove_ids.iter().any(|remove| remove == id)
        })
        .cloned()
        .collect();

    for raw in incoming {
        let item = normalize(raw);
        if !item.is_object() {
            anyhow::bail!("{} 的元素应为字符串或对象", kind);
        }

        let id = item
            .get("id")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string);

        if let Some(id) = id {
            if let Some(existing_item) = out.iter_mut().find(|old| {
                old.get("id").and_then(|v| v.as_str()).map(str::trim) == Some(id.as_str())
            }) {
                *existing_item = merge_object_values(Some(existing_item), &item);
                continue;
            }
        }

        out.push(item);
    }

    Ok(out)
}

fn merge_string_list(existing: &[Value], add: &[Value], remove: &[Value]) -> Vec<Value> {
    let mut out: Vec<String> = existing
        .iter()
        .filter_map(|v| v.as_str())
        .map(str::to_string)
        .collect();

    let remove_keys: Vec<&str> = remove.iter().filter_map(|v| v.as_str()).collect();
    out.retain(|item| !remove_keys.contains(&item.as_str()));

    for added in add.iter().filter_map(|v| v.as_str()) {
        if !out.iter().any(|item| item == added) {
            out.push(added.to_string());
        }
    }

    out.into_iter().map(Value::String).collect()
}

/// 对象数组的 add/remove：按指定 key（如 abilities.name / growth_stages.stage）匹配。
fn merge_object_list_by_key(
    existing: &[Value],
    add: &[Value],
    remove: &[Value],
    key: &str,
) -> Vec<Value> {
    let remove_keys: Vec<String> = remove
        .iter()
        .filter_map(|v| v.as_str())
        .map(|s| s.trim().to_string())
        .collect();
    let key_of = |v: &Value| -> Option<String> {
        v.get(key)
            .and_then(|x| x.as_str())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
    };

    let mut out: Vec<Value> = existing
        .iter()
        .filter(|item| match key_of(item) {
            Some(k) => !remove_keys.iter().any(|remove| remove == &k),
            None => true,
        })
        .cloned()
        .collect();

    for raw in add {
        let Some(k) = key_of(raw) else {
            out.push(raw.clone());
            continue;
        };

        if let Some(existing_item) = out.iter_mut().find(|item| key_of(item).as_ref() == Some(&k)) {
            *existing_item = merge_object_values(Some(existing_item), raw);
        } else {
            out.push(raw.clone());
        }
    }

    out
}

fn merge_aliases(existing: &[Value], add: &[String], remove: &[String]) -> Vec<String> {
    let mut out: Vec<String> = existing
        .iter()
        .filter_map(|v| v.as_str())
        .map(str::to_string)
        .collect();

    for removed in remove {
        out.retain(|item| item != removed);
    }
    for added in add {
        if !out.iter().any(|item| item == added) {
            out.push(added.clone());
        }
    }
    out
}

/// 归一化 `capabilities`：兼容模型常见的自然写法。
///
/// - 标准：`{ "skills": [...], "limitations": [...] }`
/// - 兼容：`{ "abilities": [...], "limits": [...] }`（模型实测会这样写）
/// - 兜底：直接传数组时，记为 `{ "skills": [...] }`
fn normalize_capabilities(v: &Value) -> Result<Value> {
    if let Some(list) = v.as_array() {
        return Ok(json!({ "skills": list }));
    }
    let Some(obj) = v.as_object() else {
        anyhow::bail!("capabilities 应为对象（含 skills / limitations 两个字符串数组），或字符串数组");
    };
    let mut out = obj.clone();
    if let Some(skills) = out.remove("abilities") {
        out.entry("skills".to_string()).or_insert(skills);
    }
    if let Some(limits) = out.remove("limits") {
        out.entry("limitations".to_string()).or_insert(limits);
    }
    Ok(Value::Object(out))
}

/// 归一化 `arc_potential`：模型有时偷懒只传一段字符串。
///
/// 直接字符串时按「可能的变化方向」写入 `possible_change`；对象则原样透传。
fn normalize_arc_potential(v: &Value) -> Result<Value> {
    if let Some(text) = v.as_str() {
        let text = text.trim();
        if text.is_empty() {
            return Ok(json!({}));
        }
        return Ok(json!({ "possible_change": text }));
    }
    if v.is_object() {
        return Ok(v.clone());
    }
    anyhow::bail!("arc_potential 应为对象（starting_state / possible_change / resistance），或一段描述字符串");
}

/// 归一化 `arc_stages` 的单个元素：模型有时只给阶段名，有时写成完整对象。
///
/// - 字符串 → `{ "stage": "..." }`
/// - 对象 → 原样保留，但把 `screen_weight` 的中文 / 合法写法规范成枚举值
fn normalize_arc_stage_item(raw: &Value) -> Result<Value> {
    use domain::arc_stage::ScreenWeight;

    let mut obj = match raw {
        Value::String(s) => {
            let stage = s.trim();
            if stage.is_empty() {
                anyhow::bail!("arc_stages 的元素为字符串时不能为空");
            }
            json!({ "stage": stage })
        }
        Value::Object(_) => raw.clone(),
        _ => anyhow::bail!("arc_stages 的元素应为字符串（阶段名）或对象"),
    };

    let stage = obj
        .get("stage")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string);
    let Some(stage) = stage else {
        anyhow::bail!("arc_stages 的元素缺少 stage（阶段名）");
    };
    obj["stage"] = json!(stage);

    // role 的等价键 stage_role：更中性的叫法，写势力 / 地点语境时更顺。
    // 两者都传且不一致时报错（不静默挑一个）；归一化后统一落回 role，
    // 别名键必须删掉——它没有对应列，留着就是一次静默丢弃。
    let role = obj
        .get("role")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string);
    let stage_role = obj
        .get("stage_role")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string);
    match (role, stage_role) {
        (Some(a), Some(b)) if a != b => anyhow::bail!(
            "arc_stages 元素同时给了 role 与 stage_role，且两者不同：{} / {}（它们是同一个字段，只传一个）",
            a,
            b
        ),
        (Some(a), _) => obj["role"] = json!(a),
        (None, Some(b)) => obj["role"] = json!(b),
        (None, None) => {}
    }
    if let Value::Object(map) = &mut obj {
        map.remove("stage_role");
    }

    if let Some(raw_weight) = obj.get("screen_weight").and_then(|v| v.as_str()) {
        let weight = ScreenWeight::parse(raw_weight).ok_or_else(|| {
            anyhow::anyhow!(
                "arc_stages.screen_weight 无法识别：{}（可传 Light / Medium / Heavy，或中文「轻」「中」「重」）",
                raw_weight
            )
        })?;
        obj["screen_weight"] = json!(weight.as_str());
    }

    Ok(obj)
}

/// 去掉 schema 里的 description，只保留类型 / 子字段 / 枚举等结构信息。
///
/// 用于给 `get_*_profile` 返回 `writable_fields`：让模型直接看到字段形状，
/// 又不至于把完整 schema 描述塞满上下文。
///
/// 注意一个坑：`properties` 里可能存在**名字就叫 `description` 的字段**
/// （如 `conflicts[].description`）。它的值是 `{type:...}` 对象，
/// 而 schema 元数据的 `description` 是字符串——只删字符串那个，
/// 否则模型会以为「冲突不需要描述」。
fn compact_schema_value(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut out = serde_json::Map::new();
            for (k, v) in map {
                if k == "description" && v.is_string() {
                    continue;
                }
                out.insert(k.clone(), compact_schema_value(v));
            }
            Value::Object(out)
        }
        Value::Array(items) => Value::Array(items.iter().map(compact_schema_value).collect()),
        other => other.clone(),
    }
}

/// `get_*_profile` 返回的字段契约。
///
/// 默认**只给字段名清单**；只有显式 `include_schema: true` 才给完整结构。
/// 为什么：实测完整结构 **5,944 字符/次**（占单次档案返回的 43%），
/// 而模型多数时候只需要知道"有哪些字段可以写"；真要结构时再要一次即可。
fn writable_fields_for(input: &Value, schema: &Value) -> Value {
    if input
        .get("include_schema")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
    {
        return writable_fields_meta(schema);
    }
    let names: Vec<String> = schema
        .get("properties")
        .and_then(|p| p.as_object())
        .map(|m| m.keys().filter(|k| k.as_str() != "id").cloned().collect())
        .unwrap_or_default();
    json!({
        "names": names,
        "note": "以上是全部可写字段名。需要每个字段的结构（type / enum / items）时，再调一次并传 include_schema: true"
    })
}

/// 把 update schema 的 properties 转成 `get_*_profile` 返回里的 `writable_fields`：
/// `{ 字段名: { required, type, properties, items, enum... } }`，并去掉 id 自身。
fn writable_fields_meta(schema: &Value) -> Value {
    let required: Vec<&str> = schema
        .get("required")
        .and_then(|r| r.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect())
        .unwrap_or_default();

    let mut out = serde_json::Map::new();
    if let Some(props) = schema.get("properties").and_then(|p| p.as_object()) {
        for (k, v) in props {
            if k == "id" {
                continue;
            }
            let mut entry = serde_json::Map::new();
            entry.insert(
                "required".to_string(),
                json!(required.contains(&k.as_str())),
            );
            if let Value::Object(fields) = compact_schema_value(v) {
                for (fk, fv) in fields {
                    entry.insert(fk, fv);
                }
            }
            out.insert(k.clone(), Value::Object(entry));
        }
    }
    Value::Object(out)
}

fn character_state_props() -> Value {
    json!({
        "location": { "type": "string", "description": "当前所在地" },
        "physical_state": { "type": "string", "description": "身体状态（伤势、疲劳等）" },
        "mental_state": { "type": "string", "description": "心理状态" },
        "resource_state": { "type": "string", "description": "资源状况（钱财、法器、人手）" },
        "social_state": { "type": "string", "description": "社会关系状况（处境、名声、被谁盯上）" },
        "flags": {
            "type": "array",
            "items": { "type": "string" },
            "description": "标记列表（如「重伤」「被通缉」，传了就整体替换）"
        }
    })
}

fn character_profile_update_schema() -> Value {
    let mut props = character_profile_props();
    props["id"] = json!({ "type": "string", "description": "人物实体 UUID" });
    props["append"] = json!({
        "type": "boolean",
        "description": "为 true 时把本次传入的文本字段**追加**到原值后面（默认 false = 覆盖）。长文本被输出上限截断时用它分次写，不必每次重发整段。"
    });
    json!({ "type": "object", "properties": props, "required": ["id"] })
}

fn bulk_character_profile_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "profiles": {
                "type": "array",
                "description": "要更新的角色档案数组，建议每次 3~5 条；每个元素的字段与 update_character_profile 完全一致",
                "items": character_profile_update_schema()
            }
        },
        "required": ["profiles"]
    })
}

#[async_trait]
impl AgentTool for CharacterProfileTool {
    fn name(&self) -> String {
        match self.action {
            CharacterProfileAction::Get => "get_character_profile".to_string(),
            CharacterProfileAction::Update => "update_character_profile".to_string(),
        }
    }

    fn description(&self) -> String {
        match self.action {
            CharacterProfileAction::Get => {
                "读取某个人物的设计档案（真名 / 别名 / 年龄段 / 性别 / 身份 / 外貌 / 背景 / \
                 核心性格 / 价值观 / 故事功能位 / 社会地位）。修改前先读一次，避免覆盖已聊定的内容。"
                    .to_string()
            }
            CharacterProfileAction::Update => {
                "填写 / 更新人物的设计档案。**只传需要设置的字段**，未传的保持原值。\
                 创建人物后应当随即补全档案——否则人物详情面板的「角色设定」一片空白。\
                 age_range / gender / role_in_story 可直接传中文（如「青年」「男」「主角」），\
                 也可传 schema 里列出的规范值，两种写法都会正确落库；\
                 只有无法识别的写法才会报错。\
                 drive / capabilities / arc_potential 默认按子字段 merge（只覆盖传入的子字段），\
                 传 {\"mode\":\"replace\"} 才整块替换。\
                 conflicts / secrets 默认整块替换；需要按 id 局部更新或删除条目时，\
                 传 conflicts_mode/secrets_mode=\"merge\"，并用 remove_conflict_ids / remove_secret_ids 删除。\
                 arc_stages 记录角色前中后期的身份 / 戏份 / 目标（与 arc_potential 正交），\
                 默认整块替换；需要按阶段名局部更新或删除时，传 arc_stages_mode=\"merge\"，\
                 并用 remove_arc_stages 按阶段名删除；conflicts 的 phase 用来标记该冲突从哪个阶段开始成立。\
                 aliases 可用 aliases_add / aliases_remove 只增删个别别名。"
                    .to_string()
            }
        }
    }

    fn input_schema(&self) -> Value {
        match self.action {
            CharacterProfileAction::Get => json!({
                "type": "object",
                "properties": {
                    "id": { "type": "string", "description": "人物实体 UUID" },
                "include_schema": { "type": "boolean", "description": "**可选**：为 true 时额外返回每个可写字段的完整结构（type / enum / items / properties）。默认只返回字段名清单——完整结构约 6000 字符/次，不需要就别要" }
                },
                "required": ["id"]
            }),
            CharacterProfileAction::Update => character_profile_update_schema(),
        }
    }

    async fn execute(&self, input: Value) -> Result<Value> {
        let id = parse_uuid(&input, "id")?;
        match self.action {
            CharacterProfileAction::Get => {
                let raw = self
                    .service
                    .get_character_profile(id)
                    .await?
                    .unwrap_or_else(|| json!({}));
                // data 只保留可直接回写的字段，只读内容单独放 readonly，
                // 这样模型「读到的原样写回」不会再撞上「未知字段」
                let (data, readonly) = split_readonly(&raw, &CHARACTER_READONLY_FIELDS);
                let name = require_entity(&self.service, id).await?;
                Ok(json!({
                    "ok": true,
                    "action": "get_character_profile",
                    "name": name,
                    "data": data,
                    "readonly": readonly,
                    "writable_fields": writable_fields_for(&input, &character_profile_update_schema()),
                }))
            }
            CharacterProfileAction::Update => {
                let existing = self
                    .service
                    .get_character_profile(id)
                    .await?
                    .unwrap_or_else(|| json!({}));
                let mut merged = existing.as_object().cloned().unwrap_or_default();
                // 读取结果里的只读字段（系统列 + 扩展表）不能当作档案内容回写
                for k in CHARACTER_READONLY_FIELDS {
                    merged.remove(k);
                }

                let name = require_entity(&self.service, id).await?;

                // 模型把读到的档案原样写回时，只读字段会一起进来。
                // 这里忽略它们并回执，而不是抛错——抛错会让模型反复重试却找不到出路。
                let mut ignored: Vec<String> = Vec::new();

                if let Some(obj) = input.as_object() {
                    let remove_conflict_ids = string_array(obj.get("remove_conflict_ids"));
                    let remove_secret_ids = string_array(obj.get("remove_secret_ids"));
                    let conflicts_mode = obj
                        .get("conflicts_mode")
                        .and_then(|v| v.as_str())
                        .map(|s| s.trim().to_ascii_lowercase())
                        .unwrap_or_else(|| {
                            if remove_conflict_ids.is_empty() {
                                "replace".to_string()
                            } else {
                                "merge".to_string()
                            }
                        });
                    let secrets_mode = obj
                        .get("secrets_mode")
                        .and_then(|v| v.as_str())
                        .map(|s| s.trim().to_ascii_lowercase())
                        .unwrap_or_else(|| {
                            if remove_secret_ids.is_empty() {
                                "replace".to_string()
                            } else {
                                "merge".to_string()
                            }
                        });
                    let remove_arc_stages = string_array(obj.get("remove_arc_stages"));
                    let arc_stages_mode = obj
                        .get("arc_stages_mode")
                        .and_then(|v| v.as_str())
                        .map(|s| s.trim().to_ascii_lowercase())
                        .unwrap_or_else(|| {
                            if remove_arc_stages.is_empty() {
                                "replace".to_string()
                            } else {
                                "merge".to_string()
                            }
                        });
                    let aliases_add = string_array(obj.get("aliases_add"));
                    let aliases_remove = string_array(obj.get("aliases_remove"));
                    // append=true：文本字段追加写入（长文本分次写，不必重发整段）
                    let append = obj
                        .get("append")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);

                    for (k, v) in obj {
                        if k == "id"
                            || k == "append"
                            || CHAR_CONTROL_FIELDS.contains(&k.as_str())
                            || FRAMEWORK_INJECTED_FIELDS.contains(&k.as_str())
                        {
                            continue;
                        }
                        if CHARACTER_READONLY_FIELDS.contains(&k.as_str()) {
                            ignored.push(k.clone());
                            continue;
                        }
                        // 结构字段的子字段名同样要校验：未知子键会被 serde 静默丢掉
                        agent::ensure_known_subfields(
                            k,
                            v,
                            &character_profile_update_schema(),
                        )?;
                        // 结构数组字段：默认整块替换；mode=merge 时按 id 局部更新/新增，
                        // 未提及的旧条目保留，remove_*_ids 用来删除指定条目。
                        if k == "conflicts" || k == "secrets" {
                            if !v.is_null() {
                                let incoming = v.as_array().ok_or_else(|| {
                                    anyhow::anyhow!("{} 应为数组", k)
                                })?;
                                let (mode, remove_ids) = if k == "conflicts" {
                                    (&conflicts_mode, &remove_conflict_ids)
                                } else {
                                    (&secrets_mode, &remove_secret_ids)
                                };
                                if mode == "merge" {
                                    let existing_items = merged
                                        .get(k)
                                        .and_then(|v| v.as_array())
                                        .cloned()
                                        .unwrap_or_default();
                                    let items = merge_structured_items(
                                        k,
                                        &existing_items,
                                        incoming,
                                        remove_ids,
                                    )?;
                                    merged.insert(k.clone(), Value::Array(items));
                                } else {
                                    merged.insert(k.clone(), v.clone());
                                }
                            }
                            continue;
                        }

                        // 阶段弧线：数组字段，但 merge 时按 stage 名称（而不是 id）匹配，
                        // 且无论哪种模式都先归一化元素（字符串 → {stage}，screen_weight 规范化）。
                        if k == "arc_stages" {
                            if !v.is_null() {
                                let next = apply_arc_stages(
                                    merged.get(k),
                                    v,
                                    &arc_stages_mode,
                                    &remove_arc_stages,
                                )?;
                                merged.insert(k.clone(), next);
                            }
                            continue;
                        }

                        // 结构化扩展字段：先做常见自然写法的归一化，再按 mode 做对象 merge/replace。
                        if CHAR_STRUCT_FIELDS.contains(&k.as_str()) {
                            if !v.is_null() {
                                let normalized = match k.as_str() {
                                    "drive" => normalize_drive(v)?,
                                    "capabilities" => normalize_capabilities(v)?,
                                    "arc_potential" => normalize_arc_potential(v)?,
                                    _ => v.clone(),
                                };
                                let (mode, payload) = split_struct_mode(&normalized);
                                let next = if mode == "replace" {
                                    payload
                                } else {
                                    let existing_value = merged.get(k).cloned();
                                    merge_object_values(existing_value.as_ref(), &payload)
                                };
                                merged.insert(k.clone(), next);
                            }
                            continue;
                        }
                        if CHAR_TEXT_FIELDS.contains(&k.as_str()) {
                            if let Some(s) = v.as_str() {
                                if !s.trim().is_empty() {
                                    let value = agent::append_or_replace(
                                        merged.get(k).and_then(|v| v.as_str()),
                                        s,
                                        append,
                                    );
                                    merged.insert(k.clone(), json!(value));
                                }
                            }
                            continue;
                        }
                        match k.as_str() {
                            "aliases" => {
                                if !v.is_null() {
                                    let list: Vec<String> = serde_json::from_value(v.clone())
                                        .map_err(|e| anyhow::anyhow!("aliases 应为字符串数组：{}", e))?;
                                    merged.insert("aliases".into(), json!(list));
                                }
                            }
                            "age_range" | "gender" | "role_in_story" => {
                                if let Some(s) = v.as_str() {
                                    let s = s.trim();
                                    if !s.is_empty() {
                                        // 模型常用「配角」这个口语泛称；StoryRole 没有同名枚举值，
                                        // 映射到语义最近的 Ally（辅助/盟友功能位），避免直接报错卡住。
                                        let normalized = if k == "role_in_story" {
                                            match s {
                                                "配角" | "辅助角色" => "Ally",
                                                other => other,
                                            }
                                        } else {
                                            s
                                        };
                                        let value = agent::append_or_replace(
                                            merged.get(k).and_then(|v| v.as_str()),
                                            normalized,
                                            append,
                                        );
                                        merged.insert(k.clone(), json!(value));
                                    }
                                }
                            }
                            // 模型把读到的档案原样写回时，social_position 会是完整对象；
                            // 接受它（整体替换），同时保留下面 social_position_rank 的「只改 rank」用法
                            "social_position" => {
                                if !v.is_null() {
                                    if !v.is_object() {
                                        anyhow::bail!(
                                            "social_position 应为对象（rank / authority_level / social_access）"
                                        );
                                    }
                                    merged.insert("social_position".into(), v.clone());
                                }
                            }
                            other => anyhow::bail!(
                                "未知的人物档案字段：{}。可写字段只有：{}",
                                other,
                                CHAR_WRITABLE_FIELDS.join(" / ")
                            ),
                        }
                    }

                    // 社会地位只改 rank，保留同一对象里的 authority_level / social_access
                    if let Some(rank) = input.get("social_position_rank").and_then(|v| v.as_str()) {
                        if !rank.trim().is_empty() {
                            let mut sp = merged
                                .get("social_position")
                                .cloned()
                                .filter(Value::is_object)
                                .unwrap_or_else(|| json!({}));
                            sp["rank"] = json!(rank);
                            merged.insert("social_position".to_string(), sp);
                        }
                    }

                    // 别名 add/remove：用于只增删个别别名，避免整块替换。
                    if !aliases_add.is_empty() || !aliases_remove.is_empty() {
                        let existing = merged
                            .get("aliases")
                            .and_then(|v| v.as_array())
                            .cloned()
                            .unwrap_or_default();
                        let list = merge_aliases(&existing, &aliases_add, &aliases_remove);
                        merged.insert("aliases".into(), json!(list));
                    }

                    // 只传 remove_*_ids 而不传 conflicts/secrets 时，也要能删除条目。
                    if conflicts_mode == "merge" && !remove_conflict_ids.is_empty() {
                        let existing = merged
                            .get("conflicts")
                            .and_then(|v| v.as_array())
                            .cloned()
                            .unwrap_or_default();
                        let items = merge_structured_items(
                            "conflicts",
                            &existing,
                            &[],
                            &remove_conflict_ids,
                        )?;
                        merged.insert("conflicts".into(), Value::Array(items));
                    }
                    if secrets_mode == "merge" && !remove_secret_ids.is_empty() {
                        let existing = merged
                            .get("secrets")
                            .and_then(|v| v.as_array())
                            .cloned()
                            .unwrap_or_default();
                        let items = merge_structured_items(
                            "secrets",
                            &existing,
                            &[],
                            &remove_secret_ids,
                        )?;
                        merged.insert("secrets".into(), Value::Array(items));
                    }
                    // 只传 remove_arc_stages 而不传 arc_stages 时，也要能删除阶段。
                    if arc_stages_mode == "merge" && !remove_arc_stages.is_empty() {
                        let items =
                            remove_arc_stages_only(merged.get("arc_stages"), &remove_arc_stages);
                        merged.insert("arc_stages".into(), items);
                    }
                }

                let saved = self
                    .service
                    .update_character_profile(id, Value::Object(merged))
                    .await?;
                Ok(json!({
                    "ok": true,
                    "action": "update_character_profile",
                    "name": name,
                    "ignored_readonly_fields": ignored,
                    "data": saved
                }))
            }
        }
    }
}

/// 批量更新人物档案。
///
/// 复用单条 `update_character_profile` 的解析、字段归一化、枚举容错与错误信息，
/// 所以批量更新和单条更新行为完全一致；只是把 N 次工具调用合并成一次，
/// 并允许部分失败——某一条人物档案写错不会拖垮整批。
pub struct BulkCharacterProfileTool {
    service: Arc<EntityService>,
}

impl BulkCharacterProfileTool {
    pub fn new(service: Arc<EntityService>) -> Self {
        Self { service }
    }
}

#[async_trait]
impl AgentTool for BulkCharacterProfileTool {
    fn name(&self) -> String {
        "bulk_update_character_profiles".to_string()
    }

    fn description(&self) -> String {
        "一次为**多个**人物填写 / 更新设计档案（字段与 update_character_profile 完全一致）。\
         适合「把几个角色的档案补齐」这类批量场景，避免逐条调用耗尽单轮上限。\
         建议每次 3~5 条；受单次输出长度限制，单条也别写成长篇散文。\
         某一条失败不影响其它条，返回值的 failures 会逐条说明原因。"
            .to_string()
    }

    fn input_schema(&self) -> Value {
        bulk_character_profile_schema()
    }

    async fn execute(&self, input: Value) -> Result<Value> {
        let arr = input
            .get("profiles")
            .and_then(|v| v.as_array())
            .ok_or_else(|| anyhow::anyhow!("profiles 必须是非空数组"))?;
        if arr.is_empty() {
            anyhow::bail!("profiles 不能为空");
        }

        let updater =
            CharacterProfileTool::new(CharacterProfileAction::Update, self.service.clone());
        let mut updated = Vec::new();
        let mut failures = Vec::new();

        for (index, item) in arr.iter().enumerate() {
            let id = item
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            match updater.execute(item.clone()).await {
                Ok(value) => {
                    let name = value
                        .get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    updated.push(json!({ "id": id, "name": name }));
                }
                Err(e) => failures.push(json!({
                    "index": index,
                    "id": id,
                    "error": e.to_string()
                })),
            }
        }

        Ok(json!({
            "ok": failures.is_empty(),
            "action": "bulk_update_character_profiles",
            "updated_count": updated.len(),
            "updated": updated,
            "failures": failures
        }))
    }
}

#[async_trait]
impl AgentTool for CharacterStateTool {
    fn name(&self) -> String {
        match self.action {
            CharacterStateAction::Get => "get_character_state".to_string(),
            CharacterStateAction::Update => "update_character_state".to_string(),
        }
    }

    fn description(&self) -> String {
        match self.action {
            CharacterStateAction::Get => {
                "读取某个人物当前的状态（所在地 / 身体 / 心理 / 资源 / 社会关系 / 标记）。\
                 写章节前读一次，可以避免出现「重伤角色突然生龙活虎」这类连续性问题。"
                    .to_string()
            }
            CharacterStateAction::Update => {
                "填写 / 更新人物当前的状态。**只传需要设置的字段**，未传的保持原值。\
                 剧情推进导致处境变化时应及时更新（换地点、受伤、得失资源、身份暴露）。"
                    .to_string()
            }
        }
    }

    fn input_schema(&self) -> Value {
        match self.action {
            CharacterStateAction::Get => json!({
                "type": "object",
                "properties": {
                    "id": { "type": "string", "description": "人物实体 UUID" }
                },
                "required": ["id"]
            }),
            CharacterStateAction::Update => {
                let mut props = character_state_props();
                props["id"] = json!({ "type": "string", "description": "人物实体 UUID" });
                json!({ "type": "object", "properties": props, "required": ["id"] })
            }
        }
    }

    async fn execute(&self, input: Value) -> Result<Value> {
        let id = parse_uuid(&input, "id")?;
        match self.action {
            CharacterStateAction::Get => {
                let raw = self
                    .service
                    .get_character_state(id)
                    .await?
                    .unwrap_or_else(|| json!({}));
                let (data, readonly) = split_readonly(&raw, &CHARACTER_STATE_READONLY_FIELDS);
                let name = require_entity(&self.service, id).await?;
                Ok(json!({
                    "ok": true,
                    "action": "get_character_state",
                    "name": name,
                    "data": data,
                    "readonly": readonly,
                    "writable_fields": CHAR_STATE_WRITABLE_FIELDS,
                }))
            }
            CharacterStateAction::Update => {
                let existing = self
                    .service
                    .get_character_state(id)
                    .await?
                    .unwrap_or_else(|| json!({}));
                let mut merged = existing.as_object().cloned().unwrap_or_default();
                for k in CHARACTER_STATE_READONLY_FIELDS {
                    merged.remove(k);
                }

                let name = require_entity(&self.service, id).await?;

                let mut ignored: Vec<String> = Vec::new();

                if let Some(obj) = input.as_object() {
                    for (k, v) in obj {
                        if k == "id" || FRAMEWORK_INJECTED_FIELDS.contains(&k.as_str()) {
                            continue;
                        }
                        // 只读字段（含本工具不负责写的 extra）明确忽略并回执，不抛错
                        if CHARACTER_STATE_READONLY_FIELDS.contains(&k.as_str()) {
                            ignored.push(k.clone());
                            continue;
                        }
                        if CHAR_STATE_TEXT_FIELDS.contains(&k.as_str()) {
                            if let Some(s) = v.as_str() {
                                if !s.trim().is_empty() {
                                    merged.insert(k.clone(), json!(s));
                                }
                            }
                            continue;
                        }
                        match k.as_str() {
                            "flags" => {
                                if !v.is_null() {
                                    let list: Vec<String> = serde_json::from_value(v.clone())
                                        .map_err(|e| anyhow::anyhow!("flags 应为字符串数组：{}", e))?;
                                    merged.insert("flags".into(), json!(list));
                                }
                            }
                            other => anyhow::bail!(
                                "未知的人物状态字段：{}。可写字段只有：{}",
                                other,
                                CHAR_STATE_WRITABLE_FIELDS.join(" / ")
                            ),
                        }
                    }
                }

                let saved = self
                    .service
                    .update_character_state(id, Value::Object(merged))
                    .await?;
                Ok(json!({
                    "ok": true,
                    "action": "update_character_state",
                    "name": name,
                    "ignored_readonly_fields": ignored,
                    "data": saved
                }))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn object_merge_keeps_omitted_subfields() {
        let existing = json!({"skills": ["a"], "limitations": ["b"]});
        let incoming = json!({"skills": ["a", "c"]});
        let merged = merge_object_values(Some(&existing), &incoming);
        assert_eq!(merged["skills"], json!(["a", "c"]));
        assert_eq!(merged["limitations"], json!(["b"]));
    }

    #[test]
    fn split_mode_defaults_to_merge_and_strips_mode() {
        let (mode, payload) = split_struct_mode(&json!({"mode": "replace", "skills": []}));
        assert_eq!(mode, "replace");
        assert!(payload.get("mode").is_none());
        assert!(payload.get("skills").is_some());

        let (mode, _) = split_struct_mode(&json!({"skills": []}));
        assert_eq!(mode, "merge");
    }

    #[test]
    fn structured_items_update_by_id_and_preserve_others() {
        let existing = vec![
            json!({"id": "c1", "description": "旧冲突", "conflict_type": "Internal"}),
            json!({"id": "c2", "description": "保留", "conflict_type": "External"}),
        ];
        let incoming = vec![json!({
            "id": "c1",
            "description": "新冲突"
        })];
        let out = merge_structured_items("conflicts", &existing, &incoming, &[]).unwrap();
        assert_eq!(out.len(), 2);
        assert_eq!(out[0]["id"], "c1");
        assert_eq!(out[0]["description"], "新冲突");
        assert_eq!(out[0]["conflict_type"], "Internal"); // 未传字段保留
        assert_eq!(out[1]["id"], "c2");
    }

    #[test]
    fn structured_items_remove_and_append() {
        let existing = vec![json!({"id": "c1", "description": "旧"})];
        let incoming = vec![json!({"description": "新增"})];
        let out = merge_structured_items("conflicts", &existing, &incoming, &["c1".to_string()]).unwrap();
        assert_eq!(out.len(), 1);
        assert_eq!(out[0]["description"], "新增");
    }

    #[test]
    fn object_list_add_update_remove_by_key() {
        let existing = vec![
            json!({"name": "A", "effect": "old"}),
            json!({"name": "B", "effect": "keep"}),
        ];
        let add = vec![
            json!({"name": "A", "effect": "new"}),
            json!({"name": "C", "effect": "added"}),
        ];
        let remove = vec![json!("B")];
        let out = merge_object_list_by_key(&existing, &add, &remove, "name");
        assert_eq!(out.len(), 2);
        assert_eq!(out[0]["name"], "A");
        assert_eq!(out[0]["effect"], "new");
        assert_eq!(out[1]["name"], "C");
    }

    // ---------- 实体类型清单与严格 UUID 解析 ----------

    #[test]
    fn entity_types_catalog_matches_normalizer() {
        // 清单里的每个规范名与别名都必须能被 normalize_entity_type 解析回同一个规范名，
        // 否则会出现「list_entity_types 列得出来、create_entity 却建不了」的割裂。
        for (canonical, aliases) in ENTITY_TYPES {
            assert_eq!(
                normalize_entity_type(canonical),
                Some(*canonical),
                "规范名 {} 无法自解析",
                canonical
            );
            for alias in *aliases {
                assert_eq!(
                    normalize_entity_type(alias),
                    Some(*canonical),
                    "别名 {} 应解析为 {}",
                    alias,
                    canonical
                );
            }
        }
    }

    #[test]
    fn entity_types_catalog_is_exposed_with_aliases() {
        let catalog = entity_types_catalog();
        let arr = catalog.as_array().expect("清单应为数组");
        assert_eq!(arr.len(), ENTITY_TYPES.len());
        let golden = arr
            .iter()
            .find(|t| t["entity_type"] == "golden_finger")
            .expect("应包含金手指");
        assert!(golden["aliases"]
            .as_array()
            .unwrap()
            .iter()
            .any(|a| a == "金手指"));
    }

    #[test]
    fn opt_uuid_strict_distinguishes_missing_from_invalid() {
        use serde_json::json;
        // 缺失 / null / 空串：视为「没传」
        assert_eq!(opt_uuid_strict(&json!({}), "storyline_id").unwrap(), None);
        assert_eq!(
            opt_uuid_strict(&json!({"storyline_id": null}), "storyline_id").unwrap(),
            None
        );
        assert_eq!(
            opt_uuid_strict(&json!({"storyline_id": "  "}), "storyline_id").unwrap(),
            None
        );

        // 合法值：解析出来
        let id = Uuid::new_v4();
        assert_eq!(
            opt_uuid_strict(&json!({"storyline_id": id.to_string()}), "storyline_id").unwrap(),
            Some(id)
        );

        // 非法值：**必须报错**，不能静默当成没传——
        // 否则「把伏笔挂到某条线」会静默地什么都不挂，调用方却以为挂上了。
        let err = opt_uuid_strict(&json!({"storyline_id": "abc"}), "storyline_id").unwrap_err();
        assert!(err.to_string().contains("不是合法 UUID"), "{}", err);

        let err2 = opt_uuid_strict(&json!({"storyline_id": 123}), "storyline_id").unwrap_err();
        assert!(err2.to_string().contains("UUID 字符串"), "{}", err2);
    }
}


// ============================================================
// 项目全景索引（轻量）
// ============================================================

/// 项目全景索引：一次调用回答「项目进行到什么情况」。
///
/// 为什么需要它（实测数据）：AI 原先为回答这个问题，用 3 次 `batch_call` 拉了 12 个子调用、
/// 共 **78,616 字符**的工具结果（其中 `list_entities` + `list_relations` + `list_rules`
/// 一次就 56,668 字符）。这些字符**每轮请求都要重发**，最终把模型拖到"思考 116 秒"
/// 并撞上网关超时（`error decoding response body`）。
///
/// 本工具把同一个问题压到 **3K 字符以内**：只给各类产物的**数量与分布**，
/// 明细一律用 `list_*`（有界分页）或 `get_*`（按需）去取。
pub struct ProjectIndexTool {
    project: Arc<ProjectService>,
    entity: Arc<EntityService>,
    world: Arc<WorldService>,
    narrative: Arc<NarrativeService>,
    storyline: Arc<StorylineService>,
    foreshadow: Arc<ForeshadowService>,
    history: Arc<HistoryService>,
}

impl ProjectIndexTool {
    pub fn new(
        project: Arc<ProjectService>,
        entity: Arc<EntityService>,
        world: Arc<WorldService>,
        narrative: Arc<NarrativeService>,
        storyline: Arc<StorylineService>,
        foreshadow: Arc<ForeshadowService>,
        history: Arc<HistoryService>,
    ) -> Self {
        Self {
            project,
            entity,
            world,
            narrative,
            storyline,
            foreshadow,
            history,
        }
    }
}

#[async_trait]
impl AgentTool for ProjectIndexTool {
    fn name(&self) -> String {
        "get_project_index".to_string()
    }

    fn description(&self) -> String {
        "一次拿到项目的**全景索引**：实体 / 伏笔 / 剧情线 / 叙事节点 / 最近事件的数量与分布。\
         被问「项目进行到什么情况」「现在都有什么」时**先用它**——不要用 batch_call 批量拉全量列表：\
         实测那样做一次会产生 7.8 万字符的工具结果，把上下文撑爆并导致模型思考过久、网关超时。\
         需要某个对象的正文时再用 list_*（可 limit / offset 翻页）或 get_entity / get_*_profile。"
            .to_string()
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "project_id": { "type": "string", "description": "项目 UUID（框架会自动注入当前会话的项目）" }
            },
            "required": ["project_id"]
        })
    }

    async fn execute(&self, input: Value) -> Result<Value> {
        let project_id = parse_uuid(&input, "project_id")?;

        let project = self
            .project
            .get_project(project_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("项目不存在: {}", project_id))?;

        let world = self
            .world
            .get_or_create_main_world(project_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("项目 {} 还没有主世界", project_id))?;
        let world_id = world.id;

        // 实体：按类型计数（明细用 list_entities —— 它现在有界且有 total）
        let mut by_type = serde_json::Map::new();
        let mut entity_total = 0usize;
        for t in [
            "Character",
            "Location",
            "Faction",
            "Item",
            "golden_finger",
            "Organization",
            "Creature",
            "Event",
        ] {
            let n = self.entity.list_entities(world_id, Some(t)).await?.len();
            if n > 0 {
                by_type.insert(t.to_string(), json!(n));
                entity_total += n;
            }
        }

        // 伏笔：总数 + 状态分布 + 未挂线（孤儿）数量
        let foreshadows = self.foreshadow.list_foreshadows(project_id).await?;
        let mut fs_by_status: BTreeMap<String, usize> = BTreeMap::new();
        let mut unlinked = 0usize;
        for f in &foreshadows {
            let status = f
                .get("status")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown")
                .to_string();
            *fs_by_status.entry(status).or_default() += 1;
            if f.get("storyline_id").map(|v| v.is_null()).unwrap_or(true) {
                unlinked += 1;
            }
        }

        // 剧情线：总数 + 前 20 条骨架（id + 名称）
        let storylines = self.storyline.list_storylines(project_id).await?;
        let sl_items: Vec<Value> = storylines
            .iter()
            .take(20)
            .map(|s| agent::pick_fields(s, &["id", "name"]))
            .collect();

        // 叙事节点：总数 + 类型分布
        let nodes = self.narrative.list_nodes(project_id).await?;
        let mut node_by_type: BTreeMap<String, usize> = BTreeMap::new();
        for n in &nodes {
            let t = n
                .get("node_type")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown")
                .to_string();
            *node_by_type.entry(t).or_default() += 1;
        }

        // 最近事件：只给 5 条骨架
        let events = self.history.list_events(project_id, 5).await?;
        let ev_items: Vec<Value> = events
            .iter()
            .map(|e| agent::pick_fields(e, &["id", "name", "event_time"]))
            .collect();

        Ok(json!({
            "ok": true,
            "action": "get_project_index",
            "project": agent::pick_fields(&project, &["id", "name", "status"]),
            "world_id": world_id.to_string(),
            "entities": { "total": entity_total, "by_type": by_type },
            "foreshadows": {
                "total": foreshadows.len(),
                "by_status": fs_by_status,
                "unlinked": unlinked,
            },
            "storylines": { "total": storylines.len(), "items": sl_items },
            "nodes": { "total": nodes.len(), "by_type": node_by_type },
            "recent_events": ev_items,
            "hint": "这是全景索引（只有数量与骨架）。明细用 list_entities / list_foreshadows / list_storylines 等（都支持 limit / offset 翻页）；某个对象的正文用 get_entity 或 get_*_profile。"
        }))
    }
}
