//! Runtime-facing Repository Ports.
//!
//! These traits are the dependency-inversion boundary between the runtime
//! execution layer and the concrete PostgreSQL storage in the db crate.
//!
//! runtime depends ONLY on these traits (never on db or sqlx). The db crate
//! provides the concrete implementations, injected at the application
//! composition root.

use anyhow::Result;
use async_trait::async_trait;
use uuid::Uuid;

use crate::approval::{ApprovalRecord, ApprovalTargetType};
use crate::canon::CanonRule;
use crate::entity::{Entity, Event, Fact, Relation, StateChange};
use crate::generation::{ContextPackage, GenerationTask, Skill};
use crate::knowledge::CharacterKnowledgeItem;
use crate::narrative::NarrativeNode;
use crate::state::{CurrentState, ResourceState};
use crate::storyline::Storyline;
use crate::validation::{
    CommitResponse, IssueSeverity, ProposedChange, ProposedChangeStatus, ProposedChangeType,
    ValidationIssueType, ValidationRun,
};
use crate::world::World;

/// Narrative node retrieval (project-scoped).
#[async_trait]
pub trait NarrativePort: Send + Sync {
    async fn get_node_by_id_with_project(
        &self,
        project_id: Uuid,
        node_id: Uuid,
    ) -> Result<Option<NarrativeNode>>;
    async fn list_children(&self, parent_id: Uuid) -> Result<Vec<NarrativeNode>>;
}

/// Entity retrieval (project-scoped).
#[async_trait]
pub trait EntityPort: Send + Sync {
    async fn list_entities_by_ids(&self, project_id: Uuid, ids: &[Uuid]) -> Result<Vec<Entity>>;
    async fn get_entity_by_id_with_project(
        &self,
        project_id: Uuid,
        entity_id: Uuid,
    ) -> Result<Option<Entity>>;
}

/// Current-state (projection) retrieval.
#[async_trait]
pub trait StatePort: Send + Sync {
    async fn list_current_states(
        &self,
        project_id: Uuid,
        entity_id: Uuid,
    ) -> Result<Vec<CurrentState>>;
    async fn list_current_states_batch(
        &self,
        project_id: Uuid,
        entity_ids: &[Uuid],
    ) -> Result<Vec<CurrentState>>;
}

/// Character-knowledge retrieval.
#[async_trait]
pub trait KnowledgePort: Send + Sync {
    async fn get_character_known_facts(
        &self,
        character_id: Uuid,
        project_id: Uuid,
    ) -> Result<Vec<CharacterKnowledgeItem>>;
}

/// Relation retrieval (was a raw SQL query inside ContextEngine).
#[async_trait]
pub trait RelationPort: Send + Sync {
    async fn find_relations_by_entities(
        &self,
        project_id: Uuid,
        entity_ids: &[Uuid],
    ) -> Result<Vec<Relation>>;
}

/// Story-event retrieval (was a raw SQL query inside ContextEngine).
#[async_trait]
pub trait EventPort: Send + Sync {
    async fn find_events_by_entities(
        &self,
        project_id: Uuid,
        entity_ids: &[Uuid],
    ) -> Result<Vec<Event>>;
}

/// Canon-rule + world-rule retrieval (was raw SQL inside ContextEngine/Validator).
#[async_trait]
pub trait CanonRulePort: Send + Sync {
    async fn list_canon_rules(&self, project_id: Uuid) -> Result<Vec<CanonRule>>;
    async fn get_main_world_rules_text(&self, project_id: Uuid) -> Result<Option<String>>;
}

/// Context snapshot persistence.
#[async_trait]
pub trait ContextSnapshotPort: Send + Sync {
    async fn save(&self, package: &ContextPackage) -> Result<()>;
}

/// Validation run + issue persistence (used by the Validator).
#[async_trait]
pub trait ValidationPort: Send + Sync {
    async fn create_validation_run(&self, project_id: Uuid, task_id: Uuid)
        -> Result<ValidationRun>;
    async fn update_status(&self, change_id: Uuid, status: ProposedChangeStatus) -> Result<()>;
    async fn create_issue(
        &self,
        validation_run_id: Uuid,
        proposed_change_id: Uuid,
        issue_type: ValidationIssueType,
        severity: IssueSeverity,
        message: &str,
        suggestion: Option<&str>,
    ) -> Result<()>;
    async fn update_validation_run(&self, run: &ValidationRun) -> Result<()>;
}

/// Approval-record creation (used by the Validator for warning-level changes).
#[async_trait]
pub trait ApprovalPort: Send + Sync {
    async fn create(
        &self,
        project_id: Uuid,
        target_type: ApprovalTargetType,
        target_id: Uuid,
        proposed_by: &str,
        proposal_content: serde_json::Value,
    ) -> Result<()>;
}

/// ProposedChange queries (used by the Validator).
#[async_trait]
pub trait ProposedChangeQueryPort: Send + Sync {
    async fn list_approved_changes(
        &self,
        project_id: Uuid,
        task_id: Uuid,
    ) -> Result<Vec<ProposedChange>>;
}

/// The single canonical write boundary. The concrete implementation owns the
/// database transaction; runtime only orchestrates and calls this port.
#[async_trait]
pub trait StateCommitterPort: Send + Sync {
    async fn commit(&self, project_id: Uuid, change_ids: &[Uuid]) -> Result<CommitResponse>;
}

/// Generation（生成任务）仓储端口。
///
/// 与 P1 的 runtime 端口一致：具体 SQL 留在 db 实现，
/// application 的 GenerationService 只依赖此抽象，不直接接触 db / sqlx。
#[async_trait]
pub trait GenerationRepositoryPort: Send + Sync {
    async fn list_tasks(&self, project_id: Uuid) -> Result<Vec<serde_json::Value>>;
    async fn get_task(&self, id: Uuid) -> Result<Option<serde_json::Value>>;
    async fn create_task(
        &self,
        project_id: Uuid,
        task_type: &str,
        target_id: Option<Uuid>,
        model: Option<&str>,
        parameters: serde_json::Value,
    ) -> Result<serde_json::Value>;
    async fn cancel_task(&self, id: Uuid) -> Result<()>;

    /// 取结构化 GenerationTask（供 GenerationExecutor 编排）。
    async fn get_task_struct(&self, id: Uuid) -> Result<Option<GenerationTask>>;
    /// 取 Skill（含 prompt 模板）。
    async fn get_skill_by_id(&self, id: Uuid) -> Result<Option<Skill>>;
    /// 写回任务产出。
    async fn update_task_output(&self, id: Uuid, output: serde_json::Value) -> Result<()>;
    /// 记录一次 GenerationRun（提案 十 / 十一）。context_snapshot_id 关联 ContextSnapshot（提案 十二）。
    ///
    /// `reproducibility` 携带模型 / 温度 / 检索策略 / prompt hash 等可复现元数据（ChatGPT 评审 P1）。
    async fn create_run(
        &self,
        project_id: Uuid,
        task_id: Uuid,
        context_snapshot_id: Option<Uuid>,
        llm_model: &str,
        provider: Option<&str>,
        prompt_sent: &str,
        response_received: &str,
        token_usage: Option<serde_json::Value>,
        latency_ms: Option<i64>,
        reproducibility: crate::generation::ReproducibilityMeta,
    ) -> Result<()>;
}

/// LLM 调用端口（提案 十 / 十一）。
///
/// GenerationExecutor 只依赖此抽象，具体实现在 infrastructure 中包裹 LlmClient。
#[async_trait]
pub trait LlmPort: Send + Sync {
    async fn complete(&self, system_prompt: &str, user_prompt: &str, model: &str) -> Result<String>;
}

/// 上下文快照仓储端口（提案 十二）。
///
/// GenerationExecutor 在每次执行时保存一份 ContextSnapshot，
/// 并将其 id 关联到 generation_run.context_snapshot_id。
#[async_trait]
pub trait ContextSnapshotRepositoryPort: Send + Sync {
    async fn save(&self, package: &ContextPackage) -> Result<Uuid>;
}

/// Narrative（叙事节点）仓储端口。
///
/// 与 GenerationRepositoryPort 一致：具体 SQL 留在 db 实现，
/// application 的 NarrativeService 只依赖此抽象。
#[async_trait]
pub trait NarrativeRepositoryPort: Send + Sync {
    async fn list_nodes(&self, project_id: Uuid) -> Result<Vec<serde_json::Value>>;
    async fn get_node(&self, id: Uuid) -> Result<Option<serde_json::Value>>;
    async fn create_node(
        &self,
        project_id: Uuid,
        node_type: &str,
        parent_id: Option<Uuid>,
        title: &str,
        description: Option<&str>,
        attributes: serde_json::Value,
    ) -> Result<serde_json::Value>;
    async fn update_node(
        &self,
        id: Uuid,
        title: Option<&str>,
        description: Option<&str>,
        status: Option<&str>,
    ) -> Result<serde_json::Value>;
    async fn delete_node(&self, id: Uuid) -> Result<()>;
}

/// Approval（人工审批闸门）仓储端口。
///
/// 与 GenerationRepositoryPort / NarrativeRepositoryPort 一致：
/// 具体 SQL 留在 db 实现，application 的 ApprovalService 只依赖此抽象。
#[async_trait]
pub trait ApprovalRepositoryPort: Send + Sync {
    async fn create(
        &self,
        project_id: Uuid,
        target_type: ApprovalTargetType,
        target_id: Uuid,
        proposed_by: &str,
        content: serde_json::Value,
    ) -> Result<ApprovalRecord>;
    async fn approve(&self, record_id: Uuid, reviewer_id: &str, comment: Option<&str>)
        -> Result<()>;
    async fn reject(&self, record_id: Uuid, reviewer_id: &str, comment: Option<&str>)
        -> Result<()>;
    async fn list_pending(&self, project_id: Uuid) -> Result<Vec<ApprovalRecord>>;
}

/// Proposal（提案）仓储端口。
///
/// 提案的创建、状态转换（含 CAS）与读取都集中在 port 实现里，
/// application 的 ProposalService 只负责编排，不直接接触 db / sqlx。
#[async_trait]
pub trait ProposalRepositoryPort: Send + Sync {
    async fn list_proposals(&self, project_id: Uuid) -> Result<Vec<ProposedChange>>;
    async fn get_proposal(&self, id: Uuid) -> Result<Option<ProposedChange>>;
    async fn create_proposal(
        &self,
        project_id: Uuid,
        task_id: Option<Uuid>,
        change_type: ProposedChangeType,
        target_entity_id: Uuid,
        description: &str,
        payload: serde_json::Value,
    ) -> Result<ProposedChange>;
    async fn approve_proposal(&self, id: Uuid) -> Result<ProposedChange>;
    async fn reject_proposal(&self, id: Uuid) -> Result<ProposedChange>;
}

/// Timeline（时间线）仓储端口。
///
/// 排序与冲突检测属于应用层逻辑，保留在 TimelineService；
/// 此端口只负责按项目拉取事件。
#[async_trait]
pub trait TimelineRepositoryPort: Send + Sync {
    async fn list_events_by_project(&self, project_id: Uuid) -> Result<Vec<Event>>;
}

/// Storyline（剧情线）仓储端口。
#[async_trait]
pub trait StorylineRepositoryPort: Send + Sync {
    async fn list_by_project(&self, project_id: Uuid) -> Result<Vec<Storyline>>;
    /// 列出项目全部剧情线（host 层 list_storylines 语义）。
    async fn list_storylines(&self, project_id: Uuid) -> Result<Vec<serde_json::Value>>;
    /// 创建剧情线（默认 status = Planned）。
    async fn create_storyline(
        &self,
        project_id: Uuid,
        name: &str,
        description: Option<&str>,
        importance: &str,
    ) -> Result<serde_json::Value>;
    /// 更新剧情线名称/描述（带 project 作用域校验）。
    async fn update_storyline(
        &self,
        id: Uuid,
        name: &str,
        description: Option<&str>,
    ) -> Result<serde_json::Value>;
    /// 删除剧情线（按 id）。
    async fn delete_storyline(&self, id: Uuid) -> Result<()>;
}

/// Foreshadow（伏笔）仓储端口。
#[async_trait]
pub trait ForeshadowRepositoryPort: Send + Sync {
    async fn list_foreshadows(&self, project_id: Uuid) -> Result<Vec<serde_json::Value>>;
    async fn create_foreshadow(
        &self,
        project_id: Uuid,
        name: &str,
        description: Option<&str>,
        importance: &str,
        hint_level: &str,
    ) -> Result<serde_json::Value>;
    async fn update_foreshadow(
        &self,
        id: Uuid,
        name: &str,
        description: Option<&str>,
    ) -> Result<serde_json::Value>;
    /// 删除伏笔（按 id）。
    async fn delete_foreshadow(&self, id: Uuid) -> Result<()>;
}

/// World（世界管理）仓储端口。
///
/// 这是 application 中最大的一个端口：WorldService 原本直接持有 PgPool，
/// 并自行实现 set_entity_state / record_event 的事务。P3 把这些 SQL 与事务
/// 下沉到 db 的具体实现，使 application 不再依赖 db / sqlx。
///
/// 注意：set_entity_state / record_event 直接写入 canonical state，属于
/// "系统级" 写入（与 AI 提案经 Proposal → Validate → Commit 的路径不同）。
/// 这里只是把现有实现搬进 port，不改变写入语义；未来如需统一写入边界，
/// 可再让 WorldService 改走 Proposal 路径。
#[async_trait]
pub trait WorldRepositoryPort: Send + Sync {
    async fn create_world(
        &self,
        project_id: Uuid,
        name: &str,
        description: Option<&str>,
        world_rules: Option<&str>,
        is_main: bool,
    ) -> Result<World>;
    async fn get_world(&self, world_id: Uuid) -> Result<Option<World>>;
    async fn get_main_world(&self, project_id: Uuid) -> Result<Option<World>>;
    async fn ensure_main_world(&self, project_id: Uuid, project_name: &str) -> Result<World>;
    /// 获取项目的主要世界；若不存在则按 project 名称自动创建（host 层 get_world 语义）。
    async fn get_or_create_main_world(&self, project_id: Uuid) -> Result<Option<World>>;
    /// 更新主要世界的基础字段（name / description / world_rules）。
    async fn update_main_world(
        &self,
        project_id: Uuid,
        name: Option<&str>,
        description: Option<&str>,
        world_rules: Option<&str>,
    ) -> Result<World>;

    async fn create_entity(
        &self,
        project_id: Uuid,
        world_id: Uuid,
        entity_type_name: &str,
        name: &str,
        summary: Option<&str>,
        description: Option<&str>,
        attributes: serde_json::Value,
    ) -> Result<Entity>;
    async fn get_entity(&self, project_id: Uuid, entity_id: Uuid) -> Result<Option<Entity>>;
    async fn list_entities(&self, project_id: Uuid) -> Result<Vec<Entity>>;
    async fn list_entities_by_type(
        &self,
        project_id: Uuid,
        entity_type_name: &str,
    ) -> Result<Vec<Entity>>;
    async fn create_relation(
        &self,
        project_id: Uuid,
        source_entity_id: Uuid,
        target_entity_id: Uuid,
        relation_type: &str,
        description: Option<&str>,
        attributes: serde_json::Value,
    ) -> Result<Relation>;
    async fn list_relations(&self, project_id: Uuid, entity_id: Uuid) -> Result<Vec<Relation>>;

    async fn create_fact(
        &self,
        project_id: Uuid,
        content: &str,
        category: Option<&str>,
        certainty: &str,
        related_entity_ids: &[Uuid],
    ) -> Result<Fact>;
    async fn list_facts(&self, project_id: Uuid) -> Result<Vec<Fact>>;

    async fn set_entity_state(
        &self,
        project_id: Uuid,
        entity_id: Uuid,
        state_key: &str,
        state_value: serde_json::Value,
    ) -> Result<CurrentState>;
    async fn get_entity_state(
        &self,
        project_id: Uuid,
        entity_id: Uuid,
        state_key: &str,
    ) -> Result<Option<CurrentState>>;
    async fn list_entity_states(
        &self,
        project_id: Uuid,
        entity_id: Uuid,
    ) -> Result<Vec<CurrentState>>;

    async fn upsert_resource(
        &self,
        project_id: Uuid,
        location_id: Uuid,
        resource_name: &str,
        quantity: Option<f64>,
        production_rate: Option<f64>,
        controlled_by: Option<Uuid>,
    ) -> Result<ResourceState>;
    async fn list_resources(&self, location_id: Uuid) -> Result<Vec<ResourceState>>;

    async fn record_event(
        &self,
        project_id: Uuid,
        name: &str,
        description: &str,
        event_type: Option<&str>,
        involved_entity_ids: &[Uuid],
        state_changes: Vec<StateChange>,
    ) -> Result<Event>;
}

/// Project（项目）仓储端口。
///
/// 与 P3 端口一致：具体 SQL 留在 db 实现，application 的 ProjectService 只
/// 依赖此抽象。create_project 同时自动创建项目的主要世界（与 host 层语义一致）。
#[async_trait]
pub trait ProjectRepositoryPort: Send + Sync {
    async fn list_projects(&self) -> Result<Vec<serde_json::Value>>;
    async fn get_project(&self, id: Uuid) -> Result<Option<serde_json::Value>>;
    async fn create_project(
        &self,
        name: &str,
        description: Option<&str>,
        language: Option<&str>,
    ) -> Result<serde_json::Value>;
    async fn update_project(
        &self,
        id: Uuid,
        name: Option<&str>,
        description: Option<&str>,
        status: Option<&str>,
    ) -> Result<serde_json::Value>;
    async fn delete_project(&self, id: Uuid) -> Result<()>;
}

/// Rule（canon_rule 规则）仓储端口。
#[async_trait]
pub trait RuleRepositoryPort: Send + Sync {
    async fn list_rules(&self, world_id: Uuid) -> Result<Vec<serde_json::Value>>;
    async fn create_rule(
        &self,
        world_id: Uuid,
        rule_content: &str,
        rule_level: Option<&str>,
        affected_scope: Option<&str>,
        enforcement: Option<&str>,
    ) -> Result<serde_json::Value>;
    async fn get_rule(&self, id: Uuid) -> Result<Option<serde_json::Value>>;
    async fn update_rule(
        &self,
        id: Uuid,
        rule_content: Option<&str>,
        rule_level: Option<&str>,
    ) -> Result<serde_json::Value>;
    async fn delete_rule(&self, id: Uuid) -> Result<()>;
}

/// History（event / fact / version）仓储端口。
///
/// version 相关为占位实现，仍由 host 层返回 stub，这里只覆盖有真实 SQL 的
/// event / fact 读写。
#[async_trait]
pub trait HistoryRepositoryPort: Send + Sync {
    async fn list_events(&self, project_id: Uuid, limit: i64) -> Result<Vec<serde_json::Value>>;
    async fn create_event(
        &self,
        project_id: Uuid,
        name: &str,
        description: &str,
    ) -> Result<serde_json::Value>;
    async fn list_facts(&self, project_id: Uuid) -> Result<Vec<serde_json::Value>>;
    async fn create_fact(
        &self,
        project_id: Uuid,
        content: &str,
        category: Option<&str>,
        certainty: &str,
    ) -> Result<serde_json::Value>;
}

/// Snapshot（novel_state_snapshot）仓储端口。
#[async_trait]
pub trait SnapshotRepositoryPort: Send + Sync {
    async fn list_snapshots(&self, project_id: Uuid) -> Result<Vec<serde_json::Value>>;
    async fn create_snapshot(
        &self,
        project_id: Uuid,
        name: Option<&str>,
        story_time: Option<&str>,
        world_summary: Option<&str>,
    ) -> Result<serde_json::Value>;
    async fn delete_snapshot(&self, id: Uuid) -> Result<()>;
}

/// Entity（实体 + 关系 + 角色子数据）仓储端口。
///
/// 覆盖 host 层 entity.rs 的全部真实 SQL 查询（实体 CRUD、关系 CRUD，以及
/// 角色档案 / 状态 / 知识 / 关系查询）。返回值已是 API 所需的 JSON 形状，
/// 与 GenerationRepositoryPort / NarrativeRepositoryPort 一致。
#[async_trait]
pub trait EntityRepositoryPort: Send + Sync {
    async fn list_entities(
        &self,
        world_id: Uuid,
        entity_type: Option<&str>,
    ) -> Result<Vec<serde_json::Value>>;
    async fn get_entity(&self, id: Uuid) -> Result<Option<serde_json::Value>>;
    async fn create_entity(
        &self,
        world_id: Uuid,
        entity_type_name: &str,
        name: &str,
        summary: Option<&str>,
        description: Option<&str>,
    ) -> Result<serde_json::Value>;
    async fn update_entity(
        &self,
        id: Uuid,
        name: Option<&str>,
        summary: Option<&str>,
        description: Option<&str>,
        attributes: Option<&serde_json::Value>,
    ) -> Result<serde_json::Value>;
    async fn delete_entity(&self, id: Uuid) -> Result<serde_json::Value>;

    async fn list_relations(&self, world_id: Uuid) -> Result<Vec<serde_json::Value>>;
    async fn create_relation(
        &self,
        source_entity_id: Uuid,
        target_entity_id: Uuid,
        relation_type: &str,
        description: Option<&str>,
    ) -> Result<serde_json::Value>;
    async fn delete_relation(&self, id: Uuid) -> Result<()>;

    async fn get_character_profile(&self, id: Uuid) -> Result<Option<serde_json::Value>>;
    async fn get_character_state(&self, id: Uuid) -> Result<Option<serde_json::Value>>;
    async fn update_character_profile(&self, id: Uuid, profile: serde_json::Value) -> Result<serde_json::Value>;
    async fn update_character_state(&self, id: Uuid, state: serde_json::Value) -> Result<serde_json::Value>;
    async fn get_location_profile(&self, id: Uuid) -> Result<Option<serde_json::Value>>;
    async fn upsert_location_profile(&self, id: Uuid, profile: serde_json::Value) -> Result<serde_json::Value>;
    async fn get_faction_profile(&self, id: Uuid) -> Result<Option<serde_json::Value>>;
    async fn upsert_faction_profile(&self, id: Uuid, profile: serde_json::Value) -> Result<serde_json::Value>;
    async fn get_character_knowledge(&self, id: Uuid) -> Result<Vec<serde_json::Value>>;
    async fn get_character_relationships(&self, id: Uuid) -> Result<Vec<serde_json::Value>>;
}

/// 解析对象 -> project_id 的低层读端口（仅查询，不修改）。
///
/// 用于把 world-scoped / entity-scoped 的调用收敛到 project-scoped 的
/// MutationCommand（提案要求 Command 必须携带 project_id）。
#[async_trait]
pub trait ProjectResolverPort: Send + Sync {
    async fn project_id_for_entity(&self, entity_id: Uuid) -> Result<Option<Uuid>>;
    async fn project_id_for_world(&self, world_id: Uuid) -> Result<Option<Uuid>>;
    async fn project_id_for_relation(&self, relation_id: Uuid) -> Result<Option<Uuid>>;
    async fn project_id_for_narrative_node(&self, node_id: Uuid) -> Result<Option<Uuid>>;
}
