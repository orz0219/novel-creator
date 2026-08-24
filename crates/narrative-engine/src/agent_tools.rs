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
use db::application_ports::{
    DbEntityRepositoryPort, DbForeshadowRepositoryPort, DbHistoryRepositoryPort, DbNarrativeRepositoryPort,
    DbNarrativeStateWritePort, DbProjectRepositoryPort, DbRuleRepositoryPort, DbSnapshotRepositoryPort,
    DbStorylineRepositoryPort, DbWorldRepositoryPort,
};
use db::mutation_committer::DbMutationCommitter;
use db::project_resolver::DbProjectResolverPort;
use domain::world::World;
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

fn opt_uuid(v: &Value, key: &str) -> Option<Uuid> {
    v.get(key)
        .and_then(|x| x.as_str())
        .and_then(|s| Uuid::parse_str(s).ok())
}

fn opt_i64(v: &Value, key: &str) -> Option<i64> {
    v.get(key).and_then(|x| x.as_i64())
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
        EntityAction::ReviseEntity => {
            "修改已有实体的名称 / 简介 / 描述 / 属性（仅传入字段会被更新）。这会修改已有产物。".into()
        }
        EntityAction::RetireEntity => {
            "逻辑删除（语义化软删除，绝不物理 DELETE）一个实体。删除前请先用 get_entity 确认目标 id。".into()
        }
        EntityAction::CreateRelation => "在两个实体间创建关系（relation）并落库。".into(),
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
                "world_id": { "type": "string", "description": "目标世界 UUID" },
                "name": { "type": "string" },
                "summary": { "type": "string" },
                "description": { "type": "string" }
            },
            "required": ["world_id", "name"]
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
                "relation_type": { "type": "string", "description": "如 enemy / contains / CONTROLS" },
                "description": { "type": "string" }
            },
            "required": ["source_entity_id", "target_entity_id", "relation_type"]
        }),
        EntityAction::ListEntities => json!({
            "type": "object",
            "properties": {
                "world_id": { "type": "string", "description": "世界 UUID" },
                "entity_type": { "type": "string", "description": "可选：Character / Location / Faction / Item …" }
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
                let list = self.service.list_entities(world_id, opt_str(&input, "entity_type")).await?;
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
                "project_id": { "type": "string" },
                "node_type": { "type": "string", "description": "Volume / Arc / Chapter / Scene / Beat" },
                "parent_id": { "type": "string", "description": "可选：父节点 UUID" },
                "title": { "type": "string" },
                "description": { "type": "string" },
                "attributes": { "type": "object" }
            },
            "required": ["project_id", "node_type", "title"]
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
            "properties": { "project_id": { "type": "string" } },
            "required": ["project_id"]
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
        StorylineAction::CreateStoryline => "创建跨卷剧情线（默认状态 Planned）。".into(),
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
                "project_id": { "type": "string" },
                "name": { "type": "string" },
                "description": { "type": "string" },
                "importance": { "type": "string", "description": "可选：Normal / High / Critical" }
            },
            "required": ["project_id", "name"]
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
            "properties": { "project_id": { "type": "string" } },
            "required": ["project_id"]
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
                let s = self
                    .service
                    .create_storyline(project_id, name, opt_str(&input, "description"), opt_str(&input, "importance").unwrap_or("Normal"))
                    .await?;
                Ok(json!({ "ok": true, "action": "create_storyline", "data": s }))
            }
            StorylineAction::ReviseStoryline => {
                let id = parse_uuid(&input, "id")?;
                let s = self.service.update_storyline(id, opt_str(&input, "name").ok_or_else(|| anyhow::anyhow!("name 缺失"))?, opt_str(&input, "description")).await?;
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
                "project_id": { "type": "string" },
                "name": { "type": "string" },
                "description": { "type": "string" },
                "importance": { "type": "string", "description": "可选：Normal / High / Critical" },
                "hint_level": { "type": "string", "description": "可选：提示等级" }
            },
            "required": ["project_id", "name"]
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
            "properties": { "project_id": { "type": "string" } },
            "required": ["project_id"]
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
                "world_id": { "type": "string" },
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
            "properties": { "world_id": { "type": "string" } },
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
                "project_id": { "type": "string" },
                "name": { "type": "string" },
                "story_time": { "type": "string" },
                "world_summary": { "type": "string" }
            },
            "required": ["project_id"]
        }),
        SnapshotAction::DeleteSnapshot => json!({
            "type": "object",
            "properties": { "id": { "type": "string" } },
            "required": ["id"]
        }),
        SnapshotAction::ListSnapshots => json!({
            "type": "object",
            "properties": { "project_id": { "type": "string" } },
            "required": ["project_id"]
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
// Project 聚合
// ============================================================

#[derive(Clone, Copy)]
pub enum ProjectAction {
    CreateProject,
    UpdateProject,
    DeleteProject,
    GetProject,
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
        ProjectAction::CreateProject,
        ProjectAction::UpdateProject,
        ProjectAction::DeleteProject,
        ProjectAction::GetProject,
        ProjectAction::ListProjects,
    ] {
        registry.register(Arc::new(ProjectTool::new(a, service.clone())));
    }
}

fn project_name(a: ProjectAction) -> &'static str {
    match a {
        ProjectAction::CreateProject => "create_project",
        ProjectAction::UpdateProject => "update_project",
        ProjectAction::DeleteProject => "delete_project",
        ProjectAction::GetProject => "get_project",
        ProjectAction::ListProjects => "list_projects",
    }
}

fn project_description(a: ProjectAction) -> String {
    match a {
        ProjectAction::CreateProject => "创建新小说项目（含自动创建主世界）。".into(),
        ProjectAction::UpdateProject => "修改项目元信息（名称 / 描述 / 状态）。这会修改已有产物。".into(),
        ProjectAction::DeleteProject => "删除一个项目。删除前请先用 get_project / list_projects 确认目标 id。".into(),
        ProjectAction::GetProject => "读取单一项目。".into(),
        ProjectAction::ListProjects => "列出全部项目，用于检索上下文。".into(),
    }
}

fn project_schema(a: ProjectAction) -> Value {
    match a {
        ProjectAction::CreateProject => json!({
            "type": "object",
            "properties": {
                "name": { "type": "string" },
                "description": { "type": "string" },
                "language": { "type": "string", "description": "可选：创作语言" }
            },
            "required": ["name"]
        }),
        ProjectAction::UpdateProject => json!({
            "type": "object",
            "properties": {
                "id": { "type": "string" },
                "name": { "type": "string" },
                "description": { "type": "string" },
                "status": { "type": "string" }
            },
            "required": ["id"]
        }),
        ProjectAction::DeleteProject | ProjectAction::GetProject => json!({
            "type": "object",
            "properties": { "id": { "type": "string" } },
            "required": ["id"]
        }),
        ProjectAction::ListProjects => json!({ "type": "object", "properties": {} }),
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
            ProjectAction::CreateProject => {
                let name = opt_str(&input, "name").ok_or_else(|| anyhow::anyhow!("name 缺失"))?;
                let p = self.service.create_project(name, opt_str(&input, "description"), opt_str(&input, "language")).await?;
                Ok(json!({ "ok": true, "action": "create_project", "data": p }))
            }
            ProjectAction::UpdateProject => {
                let id = parse_uuid(&input, "id")?;
                let p = self.service.update_project(id, opt_str(&input, "name"), opt_str(&input, "description"), opt_str(&input, "status")).await?;
                Ok(json!({ "ok": true, "action": "update_project", "data": p }))
            }
            ProjectAction::DeleteProject => {
                let id = parse_uuid(&input, "id")?;
                self.service.delete_project(id).await?;
                Ok(json!({ "ok": true, "action": "delete_project", "id": id.to_string() }))
            }
            ProjectAction::GetProject => {
                let id = parse_uuid(&input, "id")?;
                let p = self.service.get_project(id).await?.ok_or_else(|| anyhow::anyhow!("项目不存在: {}", id))?;
                Ok(json!({ "ok": true, "action": "get_project", "data": p }))
            }
            ProjectAction::ListProjects => {
                let list = self.service.list_projects().await?;
                Ok(json!({ "ok": true, "action": "list_projects", "data": list }))
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
            "properties": { "project_id": { "type": "string" } },
            "required": ["project_id"]
        }),
        WorldAction::UpdateMainWorld => json!({
            "type": "object",
            "properties": {
                "project_id": { "type": "string" },
                "name": { "type": "string" },
                "description": { "type": "string" },
                "world_rules": { "type": "string" }
            },
            "required": ["project_id"]
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
                "project_id": { "type": "string" },
                "name": { "type": "string" },
                "description": { "type": "string" }
            },
            "required": ["project_id", "name", "description"]
        }),
        HistoryAction::CreateFact => json!({
            "type": "object",
            "properties": {
                "project_id": { "type": "string" },
                "content": { "type": "string" },
                "category": { "type": "string", "description": "可选" },
                "certainty": { "type": "string", "description": "如 CANON / SPECULATIVE" }
            },
            "required": ["project_id", "content", "certainty"]
        }),
        HistoryAction::ListEvents => json!({
            "type": "object",
            "properties": {
                "project_id": { "type": "string" },
                "limit": { "type": "number", "description": "可选，默认 50" }
            },
            "required": ["project_id"]
        }),
        HistoryAction::ListFacts => json!({
            "type": "object",
            "properties": { "project_id": { "type": "string" } },
            "required": ["project_id"]
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
/// 调用方（main.rs）只需传入已建好的 `PgPool`；各 service 的 repo / committer /
/// resolver 在此统一构造，与 `api/*` handler 中的 `service()` 同构。
pub fn register_all_domain_tools(registry: &ToolRegistry, pool: &PgPool) {
    let committer = Arc::new(MutationCommitter::new(Arc::new(DbMutationCommitter::new(pool.clone()))));
    let resolver = Arc::new(DbProjectResolverPort::new(pool.clone()));

    let entity = Arc::new(EntityService::new(
        Arc::new(DbEntityRepositoryPort::new(pool.clone())),
        committer.clone(),
        resolver.clone(),
    ));
    register_entity_tools(registry, entity);

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

    let project = Arc::new(ProjectService::new(Arc::new(DbProjectRepositoryPort::new(pool.clone()))));
    register_project_tools(registry, project);

    let world = Arc::new(WorldService::new(Arc::new(DbWorldRepositoryPort::new(pool.clone()))));
    register_world_tools(registry, world);

    let history = Arc::new(HistoryService::new(Arc::new(DbHistoryRepositoryPort::new(pool.clone()))));
    register_history_tools(registry, history);
}
