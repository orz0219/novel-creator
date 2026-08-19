//! Domain Repository Traits - trait-only definitions without DB dependencies

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
pub trait EntityRepository: Send + Sync {
    fn get_by_id(&self, id: Uuid) -> Result<Option<Entity>>;
    fn create(&self, entity: &Entity) -> Result<Entity>;
    fn update(&self, entity: &Entity) -> Result<Entity>;
    fn delete(&self, id: Uuid) -> Result<bool>;
    fn list_by_project(&self, project_id: Uuid) -> Result<Vec<Entity>>;
    fn search_by_name(&self, project_id: Uuid, name: &str) -> Result<Vec<Entity>>;
}

/// Project Repository trait
pub trait ProjectRepository: Send + Sync {
    fn get_by_id(&self, id: Uuid) -> Result<Option<Project>>;
    fn create(&self, project: &Project) -> Result<Project>;
    fn update(&self, project: &Project) -> Result<Project>;
    fn delete(&self, id: Uuid) -> Result<bool>;
    fn list_all(&self) -> Result<Vec<Project>>;
    fn search_by_name(&self, name: &str) -> Result<Vec<Project>>;
}

/// World Repository trait
pub trait WorldRepository: Send + Sync {
    fn get_by_id(&self, id: Uuid) -> Result<Option<World>>;
    fn create(&self, world: &World) -> Result<World>;
    fn update(&self, world: &World) -> Result<World>;
    fn delete(&self, id: Uuid) -> Result<bool>;
    fn list_by_project(&self, project_id: Uuid) -> Result<Vec<World>>;
    fn get_main_world(&self, project_id: Uuid) -> Result<Option<World>>;
}

/// Narrative Repository trait
pub trait NarrativeRepository: Send + Sync {
    fn get_by_id(&self, id: Uuid) -> Result<Option<NarrativeNode>>;
    fn create(&self, node: &NarrativeNode) -> Result<NarrativeNode>;
    fn update(&self, node: &NarrativeNode) -> Result<NarrativeNode>;
    fn delete(&self, id: Uuid) -> Result<bool>;
    fn list_by_project(&self, project_id: Uuid) -> Result<Vec<NarrativeNode>>;
    fn list_children(&self, parent_id: Uuid) -> Result<Vec<NarrativeNode>>;
}

/// Knowledge Repository trait
pub trait KnowledgeRepository: Send + Sync {
    fn get_character_knowledge(&self, character_id: Uuid, project_id: Uuid) -> Result<Vec<KnowledgeState>>;
    fn record_change(&self, change: &KnowledgeChange) -> Result<()>;
    fn get_changes_by_character(&self, character_id: Uuid) -> Result<Vec<KnowledgeChange>>;
    fn get_changes_by_project(&self, project_id: Uuid) -> Result<Vec<KnowledgeChange>>;
}

/// Validation Repository trait
pub trait ValidationRepository: Send + Sync {
    fn get_proposed_change(&self, id: Uuid) -> Result<Option<ProposedChange>>;
    fn create_proposed_change(&self, change: &ProposedChange) -> Result<ProposedChange>;
    fn update_status(&self, id: Uuid, status: crate::validation::ProposedChangeStatus) -> Result<()>;
    fn get_validation_run(&self, id: Uuid) -> Result<Option<ValidationRun>>;
    fn create_validation_run(&self, run: &ValidationRun) -> Result<ValidationRun>;
    fn create_validation_issue(&self, issue: &ValidationIssue) -> Result<ValidationIssue>;
    fn get_issues_by_run(&self, run_id: Uuid) -> Result<Vec<ValidationIssue>>;
}

/// State Repository trait
///
/// StateChangeRecord 是 source of truth (append-only)。
/// CurrentState 是只读 projection，不能直接写入。
/// 只能通过 append_state_change + apply_state_change 更新。
pub trait StateRepository: Send + Sync {
    fn get_current_state(&self, project_id: Uuid, entity_id: Uuid, state_key: &str) -> Result<Option<CurrentState>>;
    fn list_current_states(&self, project_id: Uuid, entity_id: Uuid) -> Result<Vec<CurrentState>>;
    fn append_state_change(&self, change: &StateChangeRecord) -> Result<()>;
    fn get_state_history(&self, project_id: Uuid, entity_id: Uuid) -> Result<Vec<StateChangeRecord>>;
    fn apply_state_change(&self, project_id: Uuid, entity_id: Uuid, state_key: &str, new_value: serde_json::Value, expected_version: i32) -> Result<CurrentState>;
}

/// Unit of Work trait
pub trait UnitOfWorkTrait: Send + Sync {
    fn begin(&self) -> Result<Box<dyn UnitOfWorkTrait>>;
    fn commit(self: Box<Self>) -> Result<()>;
    fn rollback(self: Box<Self>) -> Result<()>;
}