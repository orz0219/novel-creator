//! Domain Repository Traits - trait-only definitions without DB dependencies
//!
//! 所有 trait 方法都是 async 的，因为后端使用 PostgreSQL (sqlx)。
//! 使用 async_trait 宏来支持 async fn in trait。

use anyhow::Result;
use uuid::Uuid;

use crate::entity::Entity;
use crate::project::Project;
use crate::world::World;
use crate::narrative::NarrativeNode;
use crate::knowledge::KnowledgeState;
use crate::ledger::KnowledgeChange;
use crate::validation::{ProposedChange, ValidationRun, ValidationIssue};
use crate::state::{CurrentState, StateChangeRecord};

/// Entity Repository trait
#[async_trait::async_trait]
pub trait EntityRepository: Send + Sync {
    async fn get_by_id(&self, id: Uuid) -> Result<Option<Entity>>;
    async fn create(&self, entity: &Entity) -> Result<Entity>;
    async fn update(&self, entity: &Entity) -> Result<Entity>;
    async fn delete(&self, id: Uuid) -> Result<bool>;
    async fn list_by_project(&self, project_id: Uuid) -> Result<Vec<Entity>>;
    async fn search_by_name(&self, project_id: Uuid, name: &str) -> Result<Vec<Entity>>;
}

/// Project Repository trait
#[async_trait::async_trait]
pub trait ProjectRepository: Send + Sync {
    async fn get_by_id(&self, id: Uuid) -> Result<Option<Project>>;
    async fn create(&self, project: &Project) -> Result<Project>;
    async fn update(&self, project: &Project) -> Result<Project>;
    async fn delete(&self, id: Uuid) -> Result<bool>;
    async fn list_all(&self) -> Result<Vec<Project>>;
    async fn search_by_name(&self, name: &str) -> Result<Vec<Project>>;
}

/// World Repository trait
#[async_trait::async_trait]
pub trait WorldRepository: Send + Sync {
    async fn get_by_id(&self, id: Uuid) -> Result<Option<World>>;
    async fn create(&self, world: &World) -> Result<World>;
    async fn update(&self, world: &World) -> Result<World>;
    async fn delete(&self, id: Uuid) -> Result<bool>;
    async fn list_by_project(&self, project_id: Uuid) -> Result<Vec<World>>;
    async fn get_main_world(&self, project_id: Uuid) -> Result<Option<World>>;
}

/// Narrative Repository trait
#[async_trait::async_trait]
pub trait NarrativeRepository: Send + Sync {
    async fn get_by_id(&self, id: Uuid) -> Result<Option<NarrativeNode>>;
    async fn create(&self, node: &NarrativeNode) -> Result<NarrativeNode>;
    async fn update(&self, node: &NarrativeNode) -> Result<NarrativeNode>;
    async fn delete(&self, id: Uuid) -> Result<bool>;
    async fn list_by_project(&self, project_id: Uuid) -> Result<Vec<NarrativeNode>>;
    async fn list_children(&self, parent_id: Uuid) -> Result<Vec<NarrativeNode>>;
}

/// Knowledge Repository trait
#[async_trait::async_trait]
pub trait KnowledgeRepository: Send + Sync {
    async fn get_character_knowledge(&self, character_id: Uuid, project_id: Uuid) -> Result<Vec<KnowledgeState>>;
    async fn record_change(&self, change: &KnowledgeChange) -> Result<()>;
    async fn get_changes_by_character(&self, character_id: Uuid) -> Result<Vec<KnowledgeChange>>;
    async fn get_changes_by_project(&self, project_id: Uuid) -> Result<Vec<KnowledgeChange>>;
}

/// Validation Repository trait
#[async_trait::async_trait]
pub trait ValidationRepository: Send + Sync {
    async fn get_proposed_change(&self, id: Uuid) -> Result<Option<ProposedChange>>;
    async fn create_proposed_change(&self, change: &ProposedChange) -> Result<ProposedChange>;
    async fn update_status(&self, id: Uuid, status: crate::validation::ProposedChangeStatus) -> Result<()>;
    async fn get_validation_run(&self, id: Uuid) -> Result<Option<ValidationRun>>;
    async fn create_validation_run(&self, run: &ValidationRun) -> Result<ValidationRun>;
    async fn create_validation_issue(&self, issue: &ValidationIssue) -> Result<ValidationIssue>;
    async fn get_issues_by_run(&self, run_id: Uuid) -> Result<Vec<ValidationIssue>>;
}

/// State Repository trait
///
/// StateChangeRecord 是 source of truth (append-only)。
/// CurrentState 是只读 projection，不能直接写入。
/// 只能通过 append_state_change + apply_state_change 更新。
#[async_trait::async_trait]
pub trait StateRepository: Send + Sync {
    async fn get_current_state(&self, project_id: Uuid, entity_id: Uuid, state_key: &str) -> Result<Option<CurrentState>>;
    async fn list_current_states(&self, project_id: Uuid, entity_id: Uuid) -> Result<Vec<CurrentState>>;
    async fn append_state_change(&self, change: &StateChangeRecord) -> Result<()>;
    async fn get_state_history(&self, project_id: Uuid, entity_id: Uuid) -> Result<Vec<StateChangeRecord>>;
    async fn apply_state_change(&self, project_id: Uuid, entity_id: Uuid, state_key: &str, new_value: serde_json::Value, expected_version: i32) -> Result<CurrentState>;
}

/// Unit of Work trait
#[async_trait::async_trait]
pub trait UnitOfWorkTrait: Send + Sync {
    async fn begin(&self) -> Result<Box<dyn UnitOfWorkTrait>>;
    async fn commit(self: Box<Self>) -> Result<()>;
    async fn rollback(self: Box<Self>) -> Result<()>;
}
