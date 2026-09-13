//! ⚠️ 本文件由 tmp/gen_sqlite_backend.py 自动生成，请勿手工编辑。
//! 如需修改逻辑，请改 PG 侧的对应文件后重新生成。

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
use sqlite_db::application_ports::{
    DbEntityRepositoryPort, DbForeshadowRepositoryPort, DbHistoryRepositoryPort, DbNarrativeRepositoryPort,
    DbNarrativeStateWritePort, DbProjectRepositoryPort, DbRuleRepositoryPort, DbSnapshotRepositoryPort,
    DbStorylineRepositoryPort, DbWorldRepositoryPort,
};
use sqlite_db::mutation_committer::DbMutationCommitter;
use sqlite_db::project_resolver::DbProjectResolverPort;
use domain::world::World;
use serde_json::{json, Value};
use sqlx::SqlitePool;
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

fn opt_uuid(v: &Value, key: &str) -> Option<Uuid> {
    v.get(key)
        .and_then(|x| x.as_str())
        .and_then(|s| Uuid::parse_str(s).ok())
}

fn opt_i64(v: &Value, key: &str) -> Option<i64> {
    v.get(key).and_then(|x| x.as_i64())
}

/// 框架在工具执行前会注入的字段（见 `agent::runtime::tool_execute`：
/// 用会话的 project_id 覆盖模型传入值以实现物理隔离）。
///
/// 它们不属于任何工具的入参契约，因此对入参做严格校验的工具必须放行它们，
/// 否则会被误判成"未知字段"。
const FRAMEWORK_INJECTED_FIELDS: [&str; 2] = ["project_id", "world_id"];

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
    GetEntity,
    ListEntities,
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
        EntityAction::GetEntity,
        EntityAction::ListEntities,
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
        EntityAction::GetEntity => "get_entity",
        EntityAction::ListEntities => "list_entities",
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
            "语义化结束（end relation）一条关系，保留历史边但不生效。删除前请先确认关系 id。".into()
        }
        EntityAction::GetEntity => "读取单一实体的当前状态（含版本号），修改 / 删除前应先调用以确认目标。".into(),
        EntityAction::ListEntities => "列出某世界下的实体（可按类型过滤），用于检索上下文。".into(),
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
                    "description": "实体类型。可传中文「人物」「地点」「势力」「物品」「组织」「生物」「事件」「金手指」，或英文 Character / Location / Faction / Item / Organization / Creature / Event / golden_finger"
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
                "attributes": { "type": "object", "description": "实体自定义属性（JSON 对象）" }
            },
            "required": ["id"]
        }),
        EntityAction::RetireEntity | EntityAction::EndRelation | EntityAction::GetEntity => json!({
            "type": "object",
            "properties": { "id": { "type": "string", "description": "目标 UUID" } },
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
                "entity_type": { "type": "string", "description": "可选：中文「人物」「地点」「势力」「物品」「金手指」，或英文 Character / Location / Faction / Item …" }
            },
            "required": ["world_id"]
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
                let e = self
                    .service
                    .update_entity(id, opt_str(&input, "name"), opt_str(&input, "summary"), opt_str(&input, "description"), input.get("attributes"))
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
                let list = self.service.list_entities(world_id, type_filter).await?;
                Ok(json!({ "ok": true, "action": "list_entities", "data": list }))
            }
        }
    }
}

// ============================================================
// Narrative 聚合
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
}

fn narrative_name(a: NarrativeAction) -> &'static str {
    match a {
        NarrativeAction::CreateNode => "create_node",
        NarrativeAction::ReviseNode => "revise_node",
        NarrativeAction::RemoveNode => "remove_node",
        NarrativeAction::GetNode => "get_node",
        NarrativeAction::ListNodes => "list_nodes",
    }
}

fn narrative_description(a: NarrativeAction) -> String {
    match a {
        NarrativeAction::CreateNode => "创建叙事节点（卷/弧/章/场/节拍），挂到某项目下（可指定父节点）。".into(),
        NarrativeAction::ReviseNode => "修改叙事节点的标题/描述/内容/状态。这会修改已有产物。".into(),
        NarrativeAction::RemoveNode => "逻辑删除（软删除 status=Deleted）一个叙事节点。删除前请先用 get_node 确认目标 id。".into(),
        NarrativeAction::GetNode => "读取单一叙事节点（修改 / 删除前应先确认目标）。".into(),
        NarrativeAction::ListNodes => "列出某项目的全部叙事节点，用于检索上下文。".into(),
    }
}

fn narrative_schema(a: NarrativeAction) -> Value {
    match a {
        NarrativeAction::CreateNode => json!({
            "type": "object",
            "properties": {
                "node_type": { "type": "string", "description": "Volume / Arc / Chapter / Scene / Beat" },
                "parent_id": { "type": "string", "description": "可选：父节点 UUID" },
                "title": { "type": "string" },
                "description": { "type": "string" },
                "attributes": { "type": "object" }
            },
            "required": ["node_type", "title"]
        }),
        NarrativeAction::ReviseNode => json!({
            "type": "object",
            "properties": {
                "id": { "type": "string" },
                "title": { "type": "string" },
                "description": { "type": "string" },
                "content": { "type": "string" },
                "status": { "type": "string" }
            },
            "required": ["id"]
        }),
        NarrativeAction::RemoveNode | NarrativeAction::GetNode => json!({
            "type": "object",
            "properties": { "id": { "type": "string" } },
            "required": ["id"]
        }),
        NarrativeAction::ListNodes => json!({
            "type": "object",
            "properties": {},
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
                let node_type = opt_str(&input, "node_type").ok_or_else(|| anyhow::anyhow!("node_type 缺失"))?;
                let title = opt_str(&input, "title").ok_or_else(|| anyhow::anyhow!("title 缺失"))?;
                let attributes = input.get("attributes").cloned().unwrap_or(json!({}));
                let node = self
                    .service
                    .create_node(project_id, node_type, opt_uuid(&input, "parent_id"), title, opt_str(&input, "description"), attributes)
                    .await?;
                Ok(json!({ "ok": true, "action": "create_node", "data": node }))
            }
            NarrativeAction::ReviseNode => {
                let id = parse_uuid(&input, "id")?;
                let node = self
                    .service
                    .update_node(id, opt_str(&input, "title"), opt_str(&input, "description"), opt_str(&input, "content"), opt_str(&input, "status"))
                    .await?;
                Ok(json!({ "ok": true, "action": "revise_node", "data": node }))
            }
            NarrativeAction::RemoveNode => {
                let id = parse_uuid(&input, "id")?;
                self.service.delete_node(id).await?;
                Ok(json!({ "ok": true, "action": "remove_node", "id": id.to_string() }))
            }
            NarrativeAction::GetNode => {
                let id = parse_uuid(&input, "id")?;
                let node = self.service.get_node(id).await?.ok_or_else(|| anyhow::anyhow!("叙事节点不存在: {}", id))?;
                Ok(json!({ "ok": true, "action": "get_node", "data": node }))
            }
            NarrativeAction::ListNodes => {
                let project_id = parse_uuid(&input, "project_id")?;
                let list = self.service.list_nodes(project_id).await?;
                Ok(json!({ "ok": true, "action": "list_nodes", "data": list }))
            }
        }
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
    ListStorylines,
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
        StorylineAction::ListStorylines,
    ] {
        registry.register(Arc::new(StorylineTool::new(a, service.clone())));
    }
}

fn storyline_name(a: StorylineAction) -> &'static str {
    match a {
        StorylineAction::CreateStoryline => "create_storyline",
        StorylineAction::ReviseStoryline => "revise_storyline",
        StorylineAction::RetireStoryline => "retire_storyline",
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
        StorylineAction::RetireStoryline => "删除一条剧情线。删除前请先用 list_storylines 确认目标 id。".into(),
        StorylineAction::ListStorylines => "列出某项目的全部剧情线，用于检索上下文。".into(),
    }
}

fn storyline_schema(a: StorylineAction) -> Value {
    match a {
        StorylineAction::CreateStoryline => json!({
            "type": "object",
            "properties": {
                "name": { "type": "string" },
                "description": { "type": "string" },
                "importance": { "type": "string", "description": "可选：Main(主线) / Important / Normal / Minor" },
                "tone": { "type": "string", "description": "明/暗线：light(明) / dark(暗)" },
                "visibility": { "type": "string", "description": "可见性：visible / hidden（暗线一般 hidden）" },
                "parent_id": { "type": "string", "description": "挂载到哪条 storyline 下（None 表示独立）" }
            },
            "required": ["name"]
        }),
        StorylineAction::ReviseStoryline => json!({
            "type": "object",
            "properties": {
                "id": { "type": "string" },
                "name": { "type": "string" },
                "description": { "type": "string" }
            },
            "required": ["id"]
        }),
        StorylineAction::RetireStoryline => json!({
            "type": "object",
            "properties": { "id": { "type": "string" } },
            "required": ["id"]
        }),
        StorylineAction::ListStorylines => json!({
            "type": "object",
            "properties": {},
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
                let s = self
                    .service
                    .create_storyline(
                        project_id,
                        name,
                        opt_str(&input, "description"),
                        opt_str(&input, "importance").unwrap_or("Normal"),
                        opt_str(&input, "tone").unwrap_or("light"),
                        opt_str(&input, "visibility").unwrap_or("visible"),
                        parent_uuid,
                    )
                    .await?;
                Ok(json!({ "ok": true, "action": "create_storyline", "data": s }))
            }
            StorylineAction::ReviseStoryline => {
                let id = parse_uuid(&input, "id")?;
                let s = self
                    .service
                    .update_storyline(
                        id,
                        opt_str(&input, "name").ok_or_else(|| anyhow::anyhow!("name 缺失"))?,
                        opt_str(&input, "description"),
                        opt_str(&input, "tone"),
                        opt_str(&input, "visibility"),
                    )
                    .await?;
                Ok(json!({ "ok": true, "action": "revise_storyline", "data": s }))
            }
            StorylineAction::RetireStoryline => {
                let id = parse_uuid(&input, "id")?;
                self.service.delete_storyline(id).await?;
                Ok(json!({ "ok": true, "action": "retire_storyline", "id": id.to_string() }))
            }
            StorylineAction::ListStorylines => {
                let project_id = parse_uuid(&input, "project_id")?;
                let list = self.service.list_storylines(project_id).await?;
                Ok(json!({ "ok": true, "action": "list_storylines", "data": list }))
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
}

impl ForeshadowTool {
    pub fn new(action: ForeshadowAction, service: Arc<ForeshadowService>) -> Self {
        Self { action, service }
    }
}

pub fn register_foreshadow_tools(registry: &ToolRegistry, service: Arc<ForeshadowService>) {
    for a in [
        ForeshadowAction::CreateForeshadow,
        ForeshadowAction::ReviseForeshadow,
        ForeshadowAction::RetireForeshadow,
        ForeshadowAction::ListForeshadows,
    ] {
        registry.register(Arc::new(ForeshadowTool::new(a, service.clone())));
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
        ForeshadowAction::CreateForeshadow => "创建伏笔（含重要度与提示等级）。".into(),
        ForeshadowAction::ReviseForeshadow => "修改伏笔名称 / 描述。这会修改已有产物。".into(),
        ForeshadowAction::RetireForeshadow => "删除一条伏笔。删除前请先用 list_foreshadows 确认目标 id。".into(),
        ForeshadowAction::ListForeshadows => "列出某项目的全部伏笔，用于检索上下文。".into(),
    }
}

fn foreshadow_schema(a: ForeshadowAction) -> Value {
    match a {
        ForeshadowAction::CreateForeshadow => json!({
            "type": "object",
            "properties": {
                "name": { "type": "string" },
                "description": { "type": "string" },
                "importance": { "type": "string", "description": "可选：Normal / High / Critical" },
                "hint_level": { "type": "string", "description": "可选：提示等级" }
            },
            "required": ["name"]
        }),
        ForeshadowAction::ReviseForeshadow => json!({
            "type": "object",
            "properties": {
                "id": { "type": "string" },
                "name": { "type": "string" },
                "description": { "type": "string" }
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
            "properties": {},
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
                let f = self
                    .service
                    .create_foreshadow(project_id, name, opt_str(&input, "description"), opt_str(&input, "importance").unwrap_or("Normal"), opt_str(&input, "hint_level").unwrap_or("low"))
                    .await?;
                Ok(json!({ "ok": true, "action": "create_foreshadow", "data": f }))
            }
            ForeshadowAction::ReviseForeshadow => {
                let id = parse_uuid(&input, "id")?;
                let f = self.service.update_foreshadow(id, opt_str(&input, "name").ok_or_else(|| anyhow::anyhow!("name 缺失"))?, opt_str(&input, "description")).await?;
                Ok(json!({ "ok": true, "action": "revise_foreshadow", "data": f }))
            }
            ForeshadowAction::RetireForeshadow => {
                let id = parse_uuid(&input, "id")?;
                self.service.delete_foreshadow(id).await?;
                Ok(json!({ "ok": true, "action": "retire_foreshadow", "id": id.to_string() }))
            }
            ForeshadowAction::ListForeshadows => {
                let project_id = parse_uuid(&input, "project_id")?;
                let list = self.service.list_foreshadows(project_id).await?;
                Ok(json!({ "ok": true, "action": "list_foreshadows", "data": list }))
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
        RuleAction::RetireRule => "删除一条世界规则。删除前请先用 get_rule / list_rules 确认目标 id。".into(),
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
            "properties": { "world_id": { "type": "string", "description": "世界 UUID" } },
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
                let list = self.service.list_rules(world_id).await?;
                Ok(json!({ "ok": true, "action": "list_rules", "data": list }))
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
        SnapshotAction::DeleteSnapshot => "删除一个快照。删除前请先用 list_snapshots 确认目标 id。".into(),
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
            "properties": {},
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
                let list = self.service.list_snapshots(project_id).await?;
                Ok(json!({ "ok": true, "action": "list_snapshots", "data": list }))
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
        ProjectAction::ListProjects => "列出所有项目（含 premise）。".into(),
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
            "properties": {},
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
                let ps = self.service.list_projects().await?;
                Ok(json!({ "ok": true, "action": "list_projects", "data": ps }))
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
    CreateFact,
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
        HistoryAction::CreateFact,
        HistoryAction::ListEvents,
        HistoryAction::ListFacts,
    ] {
        registry.register(Arc::new(HistoryTool::new(a, service.clone())));
    }
}

fn history_name(a: HistoryAction) -> &'static str {
    match a {
        HistoryAction::CreateEvent => "create_event",
        HistoryAction::CreateFact => "create_fact",
        HistoryAction::ListEvents => "list_events",
        HistoryAction::ListFacts => "list_facts",
    }
}

fn history_description(a: HistoryAction) -> String {
    match a {
        HistoryAction::CreateEvent => "创建一条历史事件（Canon 历史记录）。".into(),
        HistoryAction::CreateFact => "创建一条事实（Fact，含确定性等级）。".into(),
        HistoryAction::ListEvents => "列出某项目的历史事件（可按 limit 限制条数）。".into(),
        HistoryAction::ListFacts => "列出某项目的全部事实，用于检索上下文。".into(),
    }
}

fn history_schema(a: HistoryAction) -> Value {
    match a {
        HistoryAction::CreateEvent => json!({
            "type": "object",
            "properties": {
                "name": { "type": "string" },
                "description": { "type": "string" }
            },
            "required": ["name", "description"]
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
                "limit": { "type": "number", "description": "可选，默认 50" }
            },
            "required": []
        }),
        HistoryAction::ListFacts => json!({
            "type": "object",
            "properties": {},
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
                let e = self.service.create_event(project_id, name, desc).await?;
                Ok(json!({ "ok": true, "action": "create_event", "data": e }))
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
                let limit = opt_i64(&input, "limit").unwrap_or(50);
                let list = self.service.list_events(project_id, limit).await?;
                Ok(json!({ "ok": true, "action": "list_events", "data": list }))
            }
            HistoryAction::ListFacts => {
                let project_id = parse_uuid(&input, "project_id")?;
                let list = self.service.list_facts(project_id).await?;
                Ok(json!({ "ok": true, "action": "list_facts", "data": list }))
            }
        }
    }
}

// ============================================================
// 统一注册入口（Phase A + B）
// ============================================================

/// 在组合根构造所有聚合的 application service，并把全部领域工具注册进 `registry`。
///
/// 调用方（main.rs）只需传入已建好的 `SqlitePool`；各 service 的 repo / committer /
/// resolver 在此统一构造，与 `api/*` handler 中的 `service()` 同构。
pub fn register_all_domain_tools(registry: &ToolRegistry, pool: &SqlitePool) {
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
    registry.register(Arc::new(BulkCharacterProfileTool::new(entity)));

    let narrative = Arc::new(NarrativeService::new(
        Arc::new(DbNarrativeRepositoryPort::new(pool.clone())),
        committer.clone(),
        resolver.clone(),
    ));
    register_narrative_tools(registry, narrative);

    let storyline = Arc::new(StorylineService::new(Arc::new(DbStorylineRepositoryPort::new(pool.clone()))));
    register_storyline_tools(registry, storyline);

    let foreshadow = Arc::new(ForeshadowService::new(Arc::new(DbForeshadowRepositoryPort::new(pool.clone()))));
    register_foreshadow_tools(registry, foreshadow);

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
        world,
    ));
    register_project_tools(registry, project);

    let history = Arc::new(HistoryService::new(Arc::new(DbHistoryRepositoryPort::new(pool.clone()))));
    register_history_tools(registry, history);

    // 引导推进工具（confirm_step）：纯 pool + agent::guide::validate_step，
    // 不依赖任何 service（避免把"推进阶段"耦合到任何具体业务层）。
    register_guide_tools(registry, pool.clone());
}

// ============================================================
// Guide（引导推进）工具
// ============================================================
//
// 单一动作 confirm_step：用户在前端点"确认推进"按钮触发（不是 agent 调用）。
// 工具内部从 DB 查 MinCompleteSnapshot → 调 agent::guide::validate_step
// → 校验通过则把 project.config.current_step 推进到 next；失败则返回结构化报告。
//
// 这个工具是「引导阶段」的事实落点；与所有 service 解耦以避免引入循环依赖。

use agent::guide::{validate_step, MinCompleteSnapshot, ValidationReport};
use anyhow::Context;

pub struct GuideTool {
    pool: SqlitePool,
}

impl GuideTool {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

pub fn register_guide_tools(registry: &ToolRegistry, pool: SqlitePool) {
    registry.register(Arc::new(GuideTool::new(pool)));
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

        // 1. 取 project（含 premise、config.current_step）
        let project_row: Option<(Uuid, Option<String>, Value)> = sqlx::query_as(
            "SELECT id, premise, COALESCE(config, '{}') FROM project WHERE id = $1",
        )
        .bind(project_id)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to load project")?;
        let (pid, premise, config) = project_row.ok_or_else(|| {
            anyhow::anyhow!("项目不存在: {}", project_id)
        })?;
        let _ = pid;

        // 2. 拿主 world
        let world_row: Option<(Uuid, Option<String>)> = sqlx::query_as(
            "SELECT id, description FROM world WHERE project_id = $1 AND is_main = true LIMIT 1",
        )
        .bind(project_id)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to load main world")?;

        // 3. 组装 snapshot
        let mut snapshot = MinCompleteSnapshot::default();
        snapshot.project_premise = premise;

        if let Some((world_id, world_desc)) = world_row {
            snapshot.world_description = world_desc;
            // 查 canon_rule 条数（canon_rule 表无 status 列，按物理存在计数）
            let (rule_count,): (i64,) = sqlx::query_as(
                "SELECT COUNT(*) FROM canon_rule WHERE world_id = $1",
            )
            .bind(world_id)
            .fetch_one(&self.pool)
            .await
            .unwrap_or((0,));
            snapshot.world_rule_entity_count = rule_count;
        }

        // 4. 查 entity 数量（按 entity_type.name 筛）
        //    5 步工作流需要 5 类：Location / Faction / Item / Character / golden_finger
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
                 WHERE e.project_id = $1 AND et.name = $2 \
                 AND e.status != 'Deleted'",
            )
            .bind(project_id)
            .bind(*kind_name)
            .fetch_one(&self.pool)
            .await
            .unwrap_or((0,));
            match *counter {
                "location_entity_count" => snapshot.location_entity_count = n,
                "faction_entity_count" => snapshot.faction_entity_count = n,
                "item_entity_count" => snapshot.item_entity_count = n,
                "character_entity_count" => snapshot.character_entity_count = n,
                "golden_finger_entity_count" => snapshot.golden_finger_entity_count = n,
                _ => {}
            }
        }

        // 5. 查 golden_finger 是否有 relation 到主角（possesses 关系）
        //    简化：只要存在 任意 relation 连接 golden_finger entity 和 character entity 即认为"已连"
        let (has_rel,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM relation r \
             JOIN entity src ON src.id = r.source_entity_id \
             JOIN entity_type src_et ON src_et.id = src.entity_type_id \
             JOIN entity dst ON dst.id = r.target_entity_id \
             JOIN entity_type dst_et ON dst_et.id = dst.entity_type_id \
             WHERE r.project_id = $1 \
             AND ((src_et.name = 'golden_finger' AND dst_et.name = 'Character') \
               OR (src_et.name = 'Character' AND dst_et.name = 'golden_finger'))",
        )
        .bind(project_id)
        .fetch_one(&self.pool)
        .await
        .unwrap_or((0,));
        snapshot.golden_finger_has_relation_to_protagonist = has_rel > 0;

        // 6. character_entity_count 已在步骤 4 填好（与 Location/Faction/Item 一起按 entity_type 查）
        //    这里不需要再查。注意：'character_entity_count' 包含主角 + 所有配角。

        // 7. 查 storylines（storyline 用 importance='Main' 区分主线；
        //    状态不过滤——新创建的 storyline 默认 Planned，物理存在即视为有产物）
        let storyline_rows: Vec<(Uuid, Option<String>, String)> = sqlx::query_as(
            "SELECT id, name, COALESCE(importance, 'Normal') FROM storyline \
             WHERE project_id = $1",
        )
        .bind(project_id)
        .fetch_all(&self.pool)
        .await
        .unwrap_or_default();
        snapshot.storyline_total_count = storyline_rows.len() as i64;
        // 主线判定：importance = 'Main'（业务约定；非 Main 即副线）
        let main_count = storyline_rows
            .iter()
            .filter(|(_, _, importance)| importance == "Main")
            .count() as i64;
        snapshot.main_storyline_count = main_count;
        // 副线数 = 总数 - 主线数
        snapshot.sub_storyline_count = (snapshot.storyline_total_count - main_count).max(0);

        // 8. 查挂载数（副线中 parent_id 非空 + 父节点存在 = "已挂载"）
        let attached_count: i64 = sqlx::query_as(
            "SELECT COUNT(*) FROM storyline_relation r \
             WHERE r.project_id = $1 \
             AND EXISTS (SELECT 1 FROM storyline c WHERE c.id = r.child_id AND c.importance != 'Main')",
        )
        .bind(project_id)
        .fetch_one(&self.pool)
        .await
        .map(|(n,): (i64,)| n)
        .unwrap_or(0);
        snapshot.attached_storyline_count = attached_count;

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
                    "UPDATE project SET config = json_set(COALESCE(config, '{}'), '{current_step}', json($1)), updated_at = CURRENT_TIMESTAMP WHERE id = $2",
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
                "id": { "type": "string", "description": "地点实体 UUID" }
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
                "narrative_usage": { "type": "string", "description": "叙事用途（在故事里承担什么作用）" }
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
                    "writable_fields": writable_fields_meta(
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
                    let remove_arc_stages = string_array(obj.get("remove_arc_stages"));
                    let arc_stages_mode = arc_stages_mode_of(obj, &remove_arc_stages);
                    for (k, v) in obj {
                        if k == "id" || k == "arc_stages_mode" || k == "remove_arc_stages" {
                            continue;
                        }
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
                        // 其余只接受字符串字段；None / 空串视为"未提供"，保持原值
                        if let Some(s) = v.as_str() {
                            if !s.trim().is_empty() {
                                merged.insert(k.clone(), json!(s));
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
            let remove_arc_stages = string_array(obj.get("remove_arc_stages"));
            let arc_stages_mode = arc_stages_mode_of(obj, &remove_arc_stages);
            for (k, v) in obj {
                if k == "id" || k == "arc_stages_mode" || k == "remove_arc_stages" {
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
            "narrative_usage": { "type": "string" }
        });
        // 批量地点同样支持阶段弧线（与单条 update_location_profile 一致）
        splice_arc_stage_props(&mut item_props);
        json!({
            "type": "object",
            "properties": {
                "profiles": {
                    "type": "array",
                    "description": "要更新的地点档案数组，建议每次 3~5 条",
                    "items": {
                        "type": "object",
                        "properties": item_props,
                        "required": ["id"]
                    }
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
                "id": { "type": "string", "description": "金手指实体 UUID" }
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
                "writable_fields": writable_fields_meta(
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
    match t.to_ascii_lowercase().as_str() {
        "character" => Some("Character"),
        "人物" | "角色" => Some("Character"),
        "creature" => Some("Creature"),
        "生物" | "怪物" | "魔兽" => Some("Creature"),
        "event" => Some("Event"),
        "事件" => Some("Event"),
        "faction" => Some("Faction"),
        "势力" | "门派" | "宗门" => Some("Faction"),
        "item" => Some("Item"),
        "物品" | "道具" | "装备" | "法宝" | "灵器" => Some("Item"),
        "location" => Some("Location"),
        "地点" | "场景" | "地图" => Some("Location"),
        "organization" => Some("Organization"),
        "组织" | "团体" => Some("Organization"),
        "golden_finger" => Some("golden_finger"),
        "金手指" => Some("golden_finger"),
        _ => None,
    }
}

/// 实体类型无法识别时的统一提示。
const ENTITY_TYPE_HINT: &str = "可传 Character / Location / Faction / Item / Organization / Creature / Event / golden_finger，\
                                 或直接写中文「人物」「地点」「势力」「物品」「组织」「生物」「事件」「金手指」";

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
                    "id": { "type": "string", "description": "势力实体 UUID" }
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
                    "writable_fields": writable_fields_meta(&faction_profile_update_schema())
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
                    for (k, v) in obj {
                        if k == "id"
                            || FRAMEWORK_INJECTED_FIELDS.contains(&k.as_str())
                            || FACTION_READONLY_FIELDS.contains(&k.as_str())
                            || k == "arc_stages_mode"
                            || k == "remove_arc_stages"
                        {
                            continue;
                        }
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
                                merged.insert(k.clone(), json!(s));
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
                    "role": { "type": "string", "description": "此阶段的身份 / 功能位（人物：第一个合伙人；势力：主角靠山；地点：主角据点）" },
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
                    "id": { "type": "string", "description": "人物实体 UUID" }
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
                    "writable_fields": writable_fields_meta(&character_profile_update_schema()),
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

                    for (k, v) in obj {
                        if k == "id"
                            || CHAR_CONTROL_FIELDS.contains(&k.as_str())
                            || FRAMEWORK_INJECTED_FIELDS.contains(&k.as_str())
                        {
                            continue;
                        }
                        if CHARACTER_READONLY_FIELDS.contains(&k.as_str()) {
                            ignored.push(k.clone());
                            continue;
                        }
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
                                    merged.insert(k.clone(), json!(s));
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
                                        merged.insert(k.clone(), json!(normalized));
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
}
