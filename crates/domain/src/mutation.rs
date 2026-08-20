//! Mutation - World Canon 统一写入口的类型定义
//!
//! 所有对 World Canon 的修改（User / AI / System）都必须经由
//! [`MutationCommitterPort`] 提交一个 [`MutationCommand`]。
//!
//! 设计原则（见项目 ARCHITECTURE.md 与收口提案）：
//! - Repository 只负责低层数据库操作，不决定业务语义。
//! - 只有 MutationCommitter 能决定「何时、为何、以何种领域语义」修改 Canon。
//! - 当前状态可以更新，重要历史不能删除；所有 Canon mutation 必须走提交者。

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use uuid::Uuid;

/// 变更来源
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MutationSource {
    User,
    AI,
    System,
}

impl MutationSource {
    pub fn as_str(self) -> &'static str {
        match self {
            MutationSource::User => "user",
            MutationSource::AI => "ai",
            MutationSource::System => "system",
        }
    }
}

/// 被修改对象的类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MutationTargetType {
    Entity,
    Relation,
    Fact,
    Event,
    NarrativeNode,
    Storyline,
    Foreshadow,
    State,
}

/// 具体变更内容（与提案命令清单对应）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", content = "data")]
pub enum MutationPayload {
    CreateEntity {
        world_id: Uuid,
        entity_type: String,
        name: String,
        summary: Option<String>,
        description: Option<String>,
        attributes: serde_json::Value,
    },
    UpdateEntity {
        name: Option<String>,
        summary: Option<String>,
        description: Option<String>,
        attributes: Option<serde_json::Value>,
    },
    DeleteEntity,
    CreateRelation {
        target_entity_id: Uuid,
        relation_type: String,
        description: Option<String>,
        attributes: Option<serde_json::Value>,
    },
    /// 结束一段关系：使用 valid_until，绝不物理 DELETE（提案 五）
    EndRelation {
        valid_until: Option<String>,
    },
    CreateFact {
        content: String,
        category: Option<String>,
        related_entity_ids: Option<Vec<Uuid>>,
    },
    /// 事实生命周期：被新事实取代（提案 六）
    SupersedeFact {
        superseded_by: Uuid,
    },
    /// 事实生命周期：作废（提案 六）
    InvalidateFact,
    /// 世界事件：不可变，只允许 INSERT（提案 七）
    CreateEvent {
        name: String,
        description: String,
        event_type: Option<String>,
        event_time: Option<String>,
    },
    /// 修改角色/实体的某个状态维度（走 StateRepo，提案 八）
    SetEntityState {
        state_key: String,
        new_value: serde_json::Value,
    },
    UpdateNarrativeNode {
        title: Option<String>,
        description: Option<String>,
        attributes: Option<serde_json::Value>,
        content: Option<String>,
        status: Option<String>,
    },
    UpdateStoryline {
        title: Option<String>,
        description: Option<String>,
        attributes: Option<serde_json::Value>,
    },
    UpdateForeshadow {
        title: Option<String>,
        description: Option<String>,
        status: Option<String>,
    },
    /// 叙事节点删除：语义化软删除（status='Deleted'），绝不物理 DELETE（提案 二十二）
    DeleteNarrativeNode,
}

/// 统一 Canon mutation 命令 —— World Canon 唯一写入口的入参。
///
/// 必须携带：`project_id`、`target`、`expected_version`（乐观锁）、`source`、`payload`。
/// `command_id` 用于幂等（提案 二十七）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MutationCommand {
    /// 幂等键：同一 command_id 重复提交必须得到同一结果
    pub command_id: Uuid,
    pub project_id: Uuid,
    /// 目标对象 id（entity id / relation id / event id / narrative node id / ...）
    pub target: Uuid,
    pub target_type: MutationTargetType,
    /// 乐观锁：期望的当前版本；None 表示不做 CAS（仅系统维护任务）
    pub expected_version: Option<i32>,
    pub source: MutationSource,
    pub payload: MutationPayload,
}

impl MutationCommand {
    pub fn new(
        project_id: Uuid,
        target: Uuid,
        target_type: MutationTargetType,
        expected_version: Option<i32>,
        source: MutationSource,
        payload: MutationPayload,
    ) -> Self {
        Self {
            command_id: Uuid::new_v4(),
            project_id,
            target,
            target_type,
            expected_version,
            source,
            payload,
        }
    }
}

/// 提交结果（提案 二十六）：让 API / Agent 知道这次修改到底改了什么。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MutationCommitResult {
    pub command_id: Uuid,
    pub event_ids: Vec<Uuid>,
    pub state_change_ids: Vec<Uuid>,
    pub affected_entity_ids: Vec<Uuid>,
    /// 新创建对象的 id（entity / relation / event / fact ...）
    pub created_ids: Vec<Uuid>,
    /// 受影响对象的 (id -> 新版本号)
    pub new_versions: HashMap<Uuid, i32>,
}

impl MutationCommitResult {
    pub fn new(command_id: Uuid) -> Self {
        Self {
            command_id,
            event_ids: Vec::new(),
            state_change_ids: Vec::new(),
            affected_entity_ids: Vec::new(),
            created_ids: Vec::new(),
            new_versions: HashMap::new(),
        }
    }
}

/// Mutation Plan —— 提交前的"计划层"（ChatGPT 评审 P0：强制唯一写路径的一环）。
///
/// Proposal 不一定直接等于一次修改。例如「杀死角色 A」会派生出多条
/// [`MutationCommand`]（移除角色 / 更新关系 / 更新时间线 / 写历史事件）。
/// 这一层把"意图(Proposal)"与"具体修改步骤集合"解耦：
///
/// ```text
/// Generation → Proposal → Validator → MutationPlan → Committer → Repository
/// ```
///
/// `MutationPlan` 是经 Validator 之后、落入 Committer 之前的显式步骤集合，
/// 也是系统唯一允许变成 Canon 写操作的载体。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MutationPlan {
    pub plan_id: Uuid,
    /// 关联的提案 id（若有）
    pub proposal_id: Option<Uuid>,
    pub source: MutationSource,
    pub commands: Vec<MutationCommand>,
    /// 本次计划影响的世界列表（ChatGPT 评审 P2/B：world_version 落点显式化）。
    /// 由调用方（知道世界上下文的上层服务）给出；DB 层不负责从 command 推断 world_id。
    pub affected_worlds: Vec<Uuid>,
}

impl MutationPlan {
    pub fn new(
        source: MutationSource,
        commands: Vec<MutationCommand>,
        affected_worlds: Vec<Uuid>,
    ) -> Self {
        Self {
            plan_id: Uuid::new_v4(),
            proposal_id: None,
            source,
            commands,
            affected_worlds,
        }
    }

    pub fn from_proposal(
        proposal_id: Uuid,
        source: MutationSource,
        commands: Vec<MutationCommand>,
        affected_worlds: Vec<Uuid>,
    ) -> Self {
        Self {
            plan_id: Uuid::new_v4(),
            proposal_id: Some(proposal_id),
            source,
            commands,
            affected_worlds,
        }
    }
}

/// 一次 Canon 提交批次（ChatGPT 评审 P2/B：把 world_version 落点显式化）。
///
/// 与单条 [`MutationCommand`] 不同，批次携带「受影响的世界列表」，由调用方
/// （知道世界上下文的上层服务）显式给出——DB 层不负责从 command 推断 world_id
/// （否则 commit path 会变复杂，且以后跨 world mutation 更麻烦）。
///
/// 提交者在同一事务里：落实命令 + 为每个 affected_world 产出一个 world_version
/// （git-commit 式），保证「Canon commit == world version commit」是同一原子事实。
#[derive(Debug, Clone)]
pub struct MutationBatch {
    pub commands: Vec<MutationCommand>,
    pub affected_worlds: Vec<Uuid>,
    pub source: MutationSource,
    /// 关联的 MutationPlan id（用于 world_version.trigger_id 追溯）
    pub plan_id: Option<Uuid>,
}

/// Mutation 错误类型
#[derive(Debug, Clone, thiserror::Error)]
pub enum MutationError {
    #[error("concurrent modification on {target} (expected version {expected})")]
    ConcurrentModification { target: Uuid, expected: i32 },
    #[error("not found: {0}")]
    NotFound(String),
    #[error("validation failed: {0}")]
    Validation(String),
    #[error("unsupported mutation: {0}")]
    Unsupported(String),
    #[error("mutation conflict: {0}")]
    Conflict(String),
    #[error("internal mutation error: {0}")]
    Internal(String),
}

impl From<anyhow::Error> for MutationError {
    fn from(e: anyhow::Error) -> Self {
        MutationError::Internal(e.to_string())
    }
}

/// World Canon 唯一业务写入口。
///
/// 实现方（db 层）负责在单一事务内完成：CAS 校验 → 投影更新 →
/// StateChange → DomainEvent(system_event) → 历史写入。Repository 不被
/// Application 直接用于 mutation；只有此处可以。
#[async_trait]
pub trait MutationCommitterPort: Send + Sync {
    /// 提交单条命令（内部为单元素批量提交）
    async fn commit(
        &self,
        cmd: MutationCommand,
    ) -> Result<MutationCommitResult, MutationError>;

    /// 在同一事务中提交一批命令；任一条失败整体回滚。
    /// `batch.affected_worlds` 决定本次提交要推进哪些世界的 world_version。
    async fn commit_batch(
        &self,
        batch: MutationBatch,
    ) -> Result<Vec<MutationCommitResult>, MutationError>;

    /// 提交一个 [`MutationPlan`]（推荐的规范写入口）。
    ///
    /// 默认实现将计划展开为 [`MutationBatch`] 并提交；实现方可用它统一注入
    /// 来源 / 提案追踪 / 领域事件 / world_version，确保"任何 Canon 写操作都经过此边界"。
    async fn commit_plan(
        &self,
        plan: MutationPlan,
    ) -> Result<Vec<MutationCommitResult>, MutationError> {
        self.commit_batch(MutationBatch {
            commands: plan.commands,
            affected_worlds: plan.affected_worlds,
            source: plan.source,
            plan_id: Some(plan.plan_id),
        })
        .await
    }
}

impl MutationCommand {
    pub fn create_entity(
        project_id: Uuid,
        world_id: Uuid,
        entity_type: &str,
        name: &str,
        summary: Option<&str>,
        description: Option<&str>,
    ) -> Self {
        MutationCommand::new(
            project_id,
            world_id,
            MutationTargetType::Entity,
            None,
            MutationSource::User,
            MutationPayload::CreateEntity {
                world_id,
                entity_type: entity_type.to_string(),
                name: name.to_string(),
                summary: summary.map(|s| s.to_string()),
                description: description.map(|s| s.to_string()),
                attributes: json!({}),
            },
        )
    }

    pub fn update_entity(
        project_id: Uuid,
        entity_id: Uuid,
        expected_version: Option<i32>,
        name: Option<String>,
        summary: Option<String>,
        description: Option<String>,
        attributes: Option<serde_json::Value>,
    ) -> Self {
        MutationCommand::new(
            project_id,
            entity_id,
            MutationTargetType::Entity,
            expected_version,
            MutationSource::User,
            MutationPayload::UpdateEntity {
                name,
                summary,
                description,
                attributes,
            },
        )
    }

    pub fn delete_entity(project_id: Uuid, entity_id: Uuid, expected_version: i32) -> Self {
        MutationCommand::new(
            project_id,
            entity_id,
            MutationTargetType::Entity,
            Some(expected_version),
            MutationSource::User,
            MutationPayload::DeleteEntity,
        )
    }

    pub fn create_relation(
        project_id: Uuid,
        source_id: Uuid,
        target_id: Uuid,
        relation_type: &str,
        description: Option<&str>,
    ) -> Self {
        MutationCommand::new(
            project_id,
            source_id,
            MutationTargetType::Relation,
            None,
            MutationSource::User,
            MutationPayload::CreateRelation {
                target_entity_id: target_id,
                relation_type: relation_type.to_string(),
                description: description.map(|s| s.to_string()),
                attributes: None,
            },
        )
    }

    /// 结束关系（语义化，绝不物理 DELETE）。valid_until 为 None 表示「至今」。
    pub fn end_relation(project_id: Uuid, relation_id: Uuid, valid_until: Option<String>) -> Self {
        MutationCommand::new(
            project_id,
            relation_id,
            MutationTargetType::Relation,
            None,
            MutationSource::User,
            MutationPayload::EndRelation { valid_until },
        )
    }

    pub fn set_entity_state(
        project_id: Uuid,
        entity_id: Uuid,
        state_key: &str,
        new_value: serde_json::Value,
    ) -> Self {
        MutationCommand::new(
            project_id,
            entity_id,
            MutationTargetType::State,
            None,
            MutationSource::User,
            MutationPayload::SetEntityState {
                state_key: state_key.to_string(),
                new_value,
            },
        )
    }

    pub fn create_fact(
        project_id: Uuid,
        content: &str,
        category: Option<&str>,
        related_entity_ids: Option<Vec<Uuid>>,
    ) -> Self {
        MutationCommand::new(
            project_id,
            Uuid::nil(),
            MutationTargetType::Fact,
            None,
            MutationSource::User,
            MutationPayload::CreateFact {
                content: content.to_string(),
                category: category.map(|s| s.to_string()),
                related_entity_ids,
            },
        )
    }

    pub fn create_event(
        project_id: Uuid,
        name: &str,
        description: &str,
        event_type: Option<&str>,
        event_time: Option<&str>,
    ) -> Self {
        MutationCommand::new(
            project_id,
            Uuid::nil(),
            MutationTargetType::Event,
            None,
            MutationSource::User,
            MutationPayload::CreateEvent {
                name: name.to_string(),
                description: description.to_string(),
                event_type: event_type.map(|s| s.to_string()),
                event_time: event_time.map(|s| s.to_string()),
            },
        )
    }

    pub fn update_narrative_node(
        project_id: Uuid,
        node_id: Uuid,
        title: Option<String>,
        description: Option<String>,
        attributes: Option<serde_json::Value>,
        content: Option<String>,
        status: Option<String>,
    ) -> Self {
        MutationCommand::new(
            project_id,
            node_id,
            MutationTargetType::NarrativeNode,
            None,
            MutationSource::User,
            MutationPayload::UpdateNarrativeNode {
                title,
                description,
                attributes,
                content,
                status,
            },
        )
    }

    /// 叙事节点删除：语义化软删除（绝不物理 DELETE）。
    pub fn delete_narrative_node(project_id: Uuid, node_id: Uuid) -> Self {
        MutationCommand::new(
            project_id,
            node_id,
            MutationTargetType::NarrativeNode,
            None,
            MutationSource::User,
            MutationPayload::DeleteNarrativeNode,
        )
    }

    pub fn update_storyline(
        project_id: Uuid,
        storyline_id: Uuid,
        name: Option<String>,
        description: Option<String>,
    ) -> Self {
        MutationCommand::new(
            project_id,
            storyline_id,
            MutationTargetType::Storyline,
            None,
            MutationSource::User,
            MutationPayload::UpdateStoryline {
                title: name,
                description,
                attributes: None,
            },
        )
    }

    pub fn update_foreshadow(
        project_id: Uuid,
        foreshadow_id: Uuid,
        title: Option<String>,
        description: Option<String>,
        status: Option<String>,
    ) -> Self {
        MutationCommand::new(
            project_id,
            foreshadow_id,
            MutationTargetType::Foreshadow,
            None,
            MutationSource::User,
            MutationPayload::UpdateForeshadow {
                title,
                description,
                status,
            },
        )
    }
}
