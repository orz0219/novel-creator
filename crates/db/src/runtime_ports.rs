//! Concrete implementations of domain::ports against PostgreSQL.
//!
//! This module is the ONLY place where the runtime-facing ports meet sqlx.
//! The runtime crate never imports this module; it only depends on the
//! domain::ports traits, which are satisfied here and injected at the
//! application composition root.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use domain::ports::*;
use domain::*;
use crate::ser;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(sqlx::FromRow)]
struct RelationRow {
    id: Uuid,
    project_id: Uuid,
    source_entity_id: Uuid,
    target_entity_id: Uuid,
    relation_type: String,
    description: Option<String>,
    attributes: Option<serde_json::Value>,
    valid_from: Option<String>,
    valid_until: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

fn to_relation(r: RelationRow) -> Relation {
    Relation {
        id: r.id,
        project_id: r.project_id,
        source_entity_id: r.source_entity_id,
        target_entity_id: r.target_entity_id,
        relation_type: r.relation_type,
        description: r.description,
        attributes: r.attributes.unwrap_or_default(),
        valid_from: r.valid_from,
        valid_until: r.valid_until,
        created_at: r.created_at,
        updated_at: r.updated_at,
    }
}

#[derive(sqlx::FromRow)]
struct EventRow {
    id: Uuid,
    project_id: Uuid,
    name: String,
    description: String,
    event_type: Option<String>,
    event_time: Option<String>,
    duration: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

fn to_event(r: EventRow) -> Event {
    Event {
        id: r.id,
        project_id: r.project_id,
        name: r.name,
        description: r.description,
        event_type: r.event_type,
        timestamp: None,
        event_time: r.event_time,
        duration: r.duration,
        involved_entity_ids: Vec::new(),
        state_changes: Vec::new(),
        created_at: r.created_at,
        updated_at: r.updated_at,
    }
}

#[derive(sqlx::FromRow)]
struct ProposedChangeRow {
    id: Uuid,
    project_id: Uuid,
    task_id: Uuid,
    change_type: String,
    target_entity_id: Uuid,
    description: String,
    payload: Option<serde_json::Value>,
    status: String,
    created_at: DateTime<Utc>,
    resolved_at: Option<DateTime<Utc>>,
}

fn to_proposed_change(r: ProposedChangeRow) -> ProposedChange {
    ProposedChange {
        id: r.id,
        project_id: r.project_id,
        task_id: r.task_id,
        change_type: ser::parse_proposed_change_type(&r.change_type),
        target_entity_id: r.target_entity_id,
        description: r.description,
        payload: r.payload.unwrap_or_default(),
        status: ser::parse_proposed_change_status(&r.status),
        created_at: r.created_at,
        resolved_at: r.resolved_at,
    }
}

#[derive(sqlx::FromRow)]
struct CanonRuleRow {
    id: Uuid,
    project_id: Uuid,
    world_id: Uuid,
    rule_level: String,
    rule_content: String,
    affected_scope: String,
    enforcement: String,
    constraints: Option<serde_json::Value>,
    source: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

fn to_canon_rule(r: CanonRuleRow) -> CanonRule {
    CanonRule {
        id: r.id,
        project_id: r.project_id,
        world_id: r.world_id,
        rule_level: RuleLevel::from_str(&r.rule_level),
        rule_content: r.rule_content,
        affected_scope: r.affected_scope,
        enforcement: EnforcementAction::from_str(&r.enforcement),
        constraints: r.constraints.unwrap_or_default(),
        source: r.source,
        created_at: r.created_at,
        updated_at: r.updated_at,
    }
}

pub struct DbNarrativePort { pool: PgPool }

#[async_trait::async_trait]
impl NarrativePort for DbNarrativePort {
    async fn get_node_by_id_with_project(&self, project_id: Uuid, node_id: Uuid) -> Result<Option<NarrativeNode>> {
        crate::repos::narrative_repo::NarrativeRepo::new(self.pool.clone()).get_node_by_id_with_project(project_id, node_id).await
    }
    async fn list_children(&self, parent_id: Uuid) -> Result<Vec<NarrativeNode>> {
        crate::repos::narrative_repo::NarrativeRepo::new(self.pool.clone()).list_children(parent_id).await
    }
}

pub struct DbEntityPort { pool: PgPool }

#[async_trait::async_trait]
impl EntityPort for DbEntityPort {
    async fn list_entities_by_ids(&self, project_id: Uuid, ids: &[Uuid]) -> Result<Vec<Entity>> {
        crate::repos::entity_repo::EntityRepo::new(self.pool.clone()).list_by_ids(project_id, ids).await
    }
    async fn get_entity_by_id_with_project(&self, project_id: Uuid, entity_id: Uuid) -> Result<Option<Entity>> {
        crate::repos::entity_repo::EntityRepo::new(self.pool.clone()).get_by_id_with_project(project_id, entity_id).await
    }
}

pub struct DbStatePort { pool: PgPool }

#[async_trait::async_trait]
impl StatePort for DbStatePort {
    async fn list_current_states(&self, project_id: Uuid, entity_id: Uuid) -> Result<Vec<CurrentState>> {
        crate::repos::state_repo::StateRepo::new(self.pool.clone()).list_current_states(project_id, entity_id).await
    }
    async fn list_current_states_batch(&self, project_id: Uuid, entity_ids: &[Uuid]) -> Result<Vec<CurrentState>> {
        crate::repos::state_repo::StateRepo::new(self.pool.clone()).list_current_states_batch(project_id, entity_ids).await
    }
}

pub struct DbKnowledgePort { pool: PgPool }

#[async_trait::async_trait]
impl KnowledgePort for DbKnowledgePort {
    async fn get_character_known_facts(&self, character_id: Uuid, project_id: Uuid) -> Result<Vec<CharacterKnowledgeItem>> {
        crate::repos::knowledge_repo::KnowledgeRepo::new(self.pool.clone()).get_character_known_facts(character_id, project_id).await
    }
}

pub struct DbRelationPort { pool: PgPool }

#[async_trait::async_trait]
impl RelationPort for DbRelationPort {
    async fn find_relations_by_entities(&self, project_id: Uuid, entity_ids: &[Uuid]) -> Result<Vec<Relation>> {
        let rows = sqlx::query_as::<_, RelationRow>("SELECT id, project_id, source_entity_id, target_entity_id, relation_type, description, attributes, valid_from, valid_until, created_at, updated_at FROM relation WHERE project_id = $1 AND (source_entity_id = ANY($2) OR target_entity_id = ANY($2)) ORDER BY created_at DESC")
            .bind(project_id).bind(entity_ids).fetch_all(&self.pool).await
            .context("Failed to query relations - critical database error")?;
        Ok(rows.into_iter().map(to_relation).collect())
    }
}

pub struct DbEventPort { pool: PgPool }

#[async_trait::async_trait]
impl EventPort for DbEventPort {
    async fn find_events_by_entities(&self, project_id: Uuid, entity_ids: &[Uuid]) -> Result<Vec<Event>> {
        let rows = sqlx::query_as::<_, EventRow>("SELECT DISTINCT e.id, e.project_id, e.name, e.description, e.event_type, e.event_time, e.duration, e.created_at, e.updated_at FROM event e INNER JOIN event_entity ee ON e.id = ee.event_id WHERE e.project_id = $1 AND ee.entity_id = ANY($2) ORDER BY e.created_at DESC LIMIT 20")
            .bind(project_id).bind(entity_ids).fetch_all(&self.pool).await
            .context("Failed to query events - critical database error")?;
        Ok(rows.into_iter().map(to_event).collect())
    }
}

pub struct DbCanonRulePort { pool: PgPool }

#[async_trait::async_trait]
impl CanonRulePort for DbCanonRulePort {
    async fn list_canon_rules(&self, project_id: Uuid) -> Result<Vec<CanonRule>> {
        let rows = sqlx::query_as::<_, CanonRuleRow>("SELECT id, project_id, world_id, rule_level, rule_content, affected_scope, enforcement, constraints, source, created_at, updated_at FROM canon_rule WHERE project_id = $1")
            .bind(project_id).fetch_all(&self.pool).await
            .context("Failed to load canon rules - critical database error")?;
        Ok(rows.into_iter().map(to_canon_rule).collect())
    }
    async fn get_main_world_rules_text(&self, project_id: Uuid) -> Result<Option<String>> {
        let world_rules: Option<Option<String>> = sqlx::query_scalar::<_, Option<String>>(
            "SELECT world_rules FROM world WHERE project_id = $1 AND is_main = TRUE"
        )
        .bind(project_id)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to query world rules - critical database error")?;
        Ok(world_rules.flatten())
    }
}

pub struct DbContextSnapshotPort { pool: PgPool }

#[async_trait::async_trait]
impl ContextSnapshotPort for DbContextSnapshotPort {
    async fn save(&self, package: &ContextPackage) -> Result<()> {
        crate::repos::context_snapshot_repo::ContextSnapshotRepo::new(self.pool.clone()).save(package).await
    }
}

pub struct DbValidationPort { pool: PgPool }

#[async_trait::async_trait]
impl ValidationPort for DbValidationPort {
    async fn create_validation_run(&self, project_id: Uuid, task_id: Uuid) -> Result<ValidationRun> {
        crate::repos::validation_repo::ValidationRepo::new(self.pool.clone()).create_validation_run(project_id, task_id).await
    }
    async fn update_status(&self, change_id: Uuid, status: ProposedChangeStatus) -> Result<()> {
        crate::repos::validation_repo::ValidationRepo::new(self.pool.clone()).update_status(change_id, status).await
    }
    async fn create_issue(&self, validation_run_id: Uuid, proposed_change_id: Uuid, issue_type: ValidationIssueType, severity: IssueSeverity, message: &str, suggestion: Option<&str>) -> Result<()> {
        crate::repos::validation_repo::ValidationRepo::new(self.pool.clone()).create_issue(validation_run_id, proposed_change_id, issue_type, severity, message, suggestion).await?;
        Ok(())
    }
    async fn update_validation_run(&self, run: &ValidationRun) -> Result<()> {
        crate::repos::validation_repo::ValidationRepo::new(self.pool.clone()).update_validation_run(run).await
    }
}

pub struct DbApprovalPort { pool: PgPool }

#[async_trait::async_trait]
impl ApprovalPort for DbApprovalPort {
    async fn create(&self, project_id: Uuid, target_type: ApprovalTargetType, target_id: Uuid, proposed_by: &str, proposal_content: serde_json::Value) -> Result<()> {
        crate::repos::approval_repo::ApprovalRepo::new(self.pool.clone()).create(project_id, target_type, target_id, proposed_by, proposal_content).await?;
        Ok(())
    }
}

pub struct DbProposedChangeQueryPort { pool: PgPool }

#[async_trait::async_trait]
impl ProposedChangeQueryPort for DbProposedChangeQueryPort {
    async fn list_approved_changes(&self, project_id: Uuid, task_id: Uuid) -> Result<Vec<ProposedChange>> {
        let rows = sqlx::query_as::<_, ProposedChangeRow>("SELECT id, project_id, task_id, change_type, target_entity_id, description, payload, status, created_at, resolved_at FROM proposed_change WHERE project_id = $1 AND task_id = $2 AND status = 'Approved' ORDER BY created_at")
            .bind(project_id).bind(task_id).fetch_all(&self.pool).await
            .context("Failed to query approved changes")?;
        Ok(rows.into_iter().map(to_proposed_change).collect())
    }
}

pub struct DbStateCommitterPort { pool: PgPool }

#[async_trait::async_trait]
impl StateCommitterPort for DbStateCommitterPort {
    async fn commit(&self, project_id: Uuid, change_ids: &[Uuid]) -> Result<CommitResponse> {
        commit_changes(&self.pool, project_id, change_ids).await
    }
}

/// Transactional commit of approved ProposedChanges. All changes are committed
/// in a single BEGIN/COMMIT transaction; any failure rolls back. This is the
/// ONLY place that mutates canonical world state.
async fn commit_changes(pool: &PgPool, project_id: Uuid, change_ids: &[Uuid]) -> Result<CommitResponse> {
    let mut tx = pool.begin().await.context("Failed to begin transaction")?;
    let mut results = Vec::new();
    let mut event_ids = Vec::new();
    // Intra-batch CAS guard: two changes in the *same* commit batch that target the
    // same (entity_id, state_key) are mutually exclusive — the second one must fail
    // so the whole transaction rolls back (last-write-wins across an atomic commit is
    // non-deterministic and must be rejected at commit time).
    let mut committed_state_keys: std::collections::HashSet<(Uuid, String)> = std::collections::HashSet::new();

    for change_id in change_ids {
        let change = crate::repos::validation_repo::ValidationRepo::get_proposed_change_by_id_for_update_tx(&mut *tx, *change_id).await?
            .ok_or_else(|| anyhow::anyhow!("ProposedChange {} not found in database", change_id))?;

        if change.status != ProposedChangeStatus::Approved {
            return Err(anyhow::anyhow!("Cannot commit ProposedChange {}: status is {:?}, expected Approved", change.id, change.status));
        }
        if change.project_id != project_id {
            return Err(anyhow::anyhow!("Cannot commit ProposedChange {}: project_id {} does not match expected {}", change.id, change.project_id, project_id));
        }

        let event = DomainEvent::new(DomainEventType::ProposalCommitted, project_id, Some(change.target_entity_id), serde_json::json!({"proposed_change_id": change.id, "change_type": format!("{:?}", change.change_type), "payload": change.payload}));

        sqlx::query("INSERT INTO system_events (id, event_type, project_id, entity_id, data, source, created_at) VALUES ($1, $2, $3, $4, $5, $6, $7)")
            .bind(event.id).bind(format!("{:?}", event.event_type)).bind(event.project_id).bind(event.entity_id).bind(&event.data).bind(&event.metadata.source).bind(event.created_at)
            .execute(&mut *tx).await.context("Failed to persist DomainEvent")?;

        event_ids.push(event.id);

        // Dispatch on the authoritative change_type (the stored JSON payload has no
        // serde discriminator, so it cannot be deserialized into ChangePayload directly).
        match change.change_type {
            ProposedChangeType::StateChange => {
                #[derive(serde::Deserialize)]
                struct StateChangePayload { state_key: String, new_value: serde_json::Value }
                let p: StateChangePayload = serde_json::from_value(change.payload.clone())
                    .map_err(|e| anyhow::anyhow!("Invalid StateChange payload for {}: {}", change.id, e))?;
                let state_key = (change.target_entity_id, p.state_key.clone());
                if committed_state_keys.contains(&state_key) {
                    return Err(anyhow::anyhow!(
                        "CAS conflict in commit batch: ProposedChange {} targets state_key '{}' on entity {} which was already modified by another change in this same commit",
                        change.id, p.state_key, change.target_entity_id
                    ));
                }
                committed_state_keys.insert(state_key);
                let entity = crate::repos::entity_repo::EntityRepo::get_by_id_with_project_tx(&mut *tx, project_id, change.target_entity_id).await?;
                if entity.is_none() {
                    return Err(anyhow::anyhow!("Cannot commit ProposedChange {}: target entity {} not found in project {}", change.id, change.target_entity_id, project_id));
                }
                let (record, new_version) = crate::repos::state_repo::StateRepo::commit_state_change_tx(&mut *tx, project_id, Some(event.id), "STATE_CHANGE", change.target_entity_id, &p.state_key, p.new_value, Some("committer")).await?;
                let rows_affected = crate::repos::validation_repo::ValidationRepo::update_status_with_guard_tx(&mut *tx, change.id, ProposedChangeStatus::Applied, ProposedChangeStatus::Approved).await?;
                if rows_affected == 0 {
                    return Err(anyhow::anyhow!("Concurrent modification detected for ProposedChange {}", change.id));
                }
                results.push(CommitResult::StateChange { record, new_version });
            }
            ProposedChangeType::EntityCreate => {
                #[derive(serde::Deserialize)]
                struct EntityCreatePayload { entity_type: String, name: String, attributes: serde_json::Value }
                let p: EntityCreatePayload = serde_json::from_value(change.payload.clone())
                    .map_err(|e| anyhow::anyhow!("Invalid EntityCreate payload for {}: {}", change.id, e))?;
                let entity_type_obj = crate::repos::entity_repo::EntityTypeRepo::ensure_tx(&mut *tx, &p.entity_type, None).await?;
                let world_id = sqlx::query_scalar::<_, Uuid>("SELECT id FROM world WHERE project_id = $1 AND is_main = TRUE LIMIT 1").bind(project_id).fetch_one(&mut *tx).await.context("No main world found for project")?;
                let entity = crate::repos::entity_repo::EntityRepo::create_tx(&mut *tx, project_id, world_id, entity_type_obj.id, &p.name, None, None, p.attributes).await?;
                let rows_affected = crate::repos::validation_repo::ValidationRepo::update_status_with_guard_tx(&mut *tx, change.id, ProposedChangeStatus::Applied, ProposedChangeStatus::Approved).await?;
                if rows_affected == 0 {
                    return Err(anyhow::anyhow!("Concurrent modification detected for ProposedChange {}", change.id));
                }
                results.push(CommitResult::EntityCreated { entity_id: entity.id, entity_name: entity.name });
            }
            ProposedChangeType::RelationCreate => {
                #[derive(serde::Deserialize)]
                struct RelationCreatePayload { target_entity_id: Uuid, relation_type: String, attributes: serde_json::Value }
                let p: RelationCreatePayload = serde_json::from_value(change.payload.clone())
                    .map_err(|e| anyhow::anyhow!("Invalid RelationCreate payload for {}: {}", change.id, e))?;
                let source = crate::repos::entity_repo::EntityRepo::get_by_id_with_project_tx(&mut *tx, project_id, change.target_entity_id).await?;
                if source.is_none() {
                    return Err(anyhow::anyhow!("Cannot commit ProposedChange {}: source entity {} not found in project {}", change.id, change.target_entity_id, project_id));
                }
                let target = crate::repos::entity_repo::EntityRepo::get_by_id_with_project_tx(&mut *tx, project_id, p.target_entity_id).await?;
                if target.is_none() {
                    return Err(anyhow::anyhow!("Cannot commit ProposedChange {}: target entity {} not found", change.id, p.target_entity_id));
                }
                let relation = crate::repos::entity_repo::RelationRepo::create_tx(&mut *tx, project_id, change.target_entity_id, p.target_entity_id, &p.relation_type, None, p.attributes).await?;
                let rows_affected = crate::repos::validation_repo::ValidationRepo::update_status_with_guard_tx(&mut *tx, change.id, ProposedChangeStatus::Applied, ProposedChangeStatus::Approved).await?;
                if rows_affected == 0 {
                    return Err(anyhow::anyhow!("Concurrent modification detected for ProposedChange {}", change.id));
                }
                results.push(CommitResult::RelationCreated { relation_id: relation.id, source_entity_id: relation.source_entity_id, target_entity_id: relation.target_entity_id, relation_type: relation.relation_type });
            }
            _ => {
                return Err(anyhow::anyhow!("Unsupported change payload type for ProposedChange {}", change.id));
            }
        }
    }

    tx.commit().await.context("Failed to commit transaction")?;

    tracing::info!("Committed {} changes with {} events in a single transaction", results.len(), event_ids.len());

    Ok(CommitResponse { project_id, results, events: event_ids, committed_at: Utc::now() })
}

// ---------------------------------------------------------------------------
// Constructors: each port is built from a PgPool. These are the only place the
// ports meet sqlx, so the runtime / composition root can build them directly.
// ---------------------------------------------------------------------------

impl DbNarrativePort {
    pub fn new(pool: PgPool) -> Self { Self { pool } }
}
impl DbEntityPort {
    pub fn new(pool: PgPool) -> Self { Self { pool } }
}
impl DbStatePort {
    pub fn new(pool: PgPool) -> Self { Self { pool } }
}
impl DbKnowledgePort {
    pub fn new(pool: PgPool) -> Self { Self { pool } }
}
impl DbRelationPort {
    pub fn new(pool: PgPool) -> Self { Self { pool } }
}
impl DbEventPort {
    pub fn new(pool: PgPool) -> Self { Self { pool } }
}
impl DbCanonRulePort {
    pub fn new(pool: PgPool) -> Self { Self { pool } }
}
impl DbContextSnapshotPort {
    pub fn new(pool: PgPool) -> Self { Self { pool } }
}
impl DbValidationPort {
    pub fn new(pool: PgPool) -> Self { Self { pool } }
}
impl DbApprovalPort {
    pub fn new(pool: PgPool) -> Self { Self { pool } }
}
impl DbProposedChangeQueryPort {
    pub fn new(pool: PgPool) -> Self { Self { pool } }
}
impl DbStateCommitterPort {
    pub fn new(pool: PgPool) -> Self { Self { pool } }
}
