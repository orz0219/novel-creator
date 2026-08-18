//! Validator - 变更验证
//!
//! 核心原则：AI 只能提出 ProposedChange，所有变更必须经过验证并事务化提交。

use anyhow::{Context, Result};
use chrono::Utc;
use db::connection::Database;
use db::repos::{entity_repo, state_repo, validation_repo};
use domain::*;
use uuid::Uuid;

/// Validator - 变更验证器
pub struct Validator<'a> {
    db: &'a Database,
}

impl<'a> Validator<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    /// 验证一批 ProposedChange
    pub fn validate_changes(
        &self,
        project_id: Uuid,
        task_id: Uuid,
        changes: &[ProposedChange],
    ) -> Result<ValidationRun> {
        let val_repo = validation_repo::ValidationRepo::new(self.db);
        let entity_repo = entity_repo::EntityRepo::new(self.db);
        let state_repo = state_repo::StateRepo::new(self.db);

        let mut run = val_repo.create_validation_run(project_id, task_id)?;
        let mut approved = 0;
        let mut rejected = 0;

        for change in changes {
            let issues = self.validate_single_change(change, &entity_repo, &state_repo)?;

            let has_critical = issues.iter().any(|i| i.severity == IssueSeverity::Critical);
            let has_warning = issues.iter().any(|i| i.severity == IssueSeverity::Warning);

            if has_critical {
                val_repo.update_status(change.id, ProposedChangeStatus::Rejected)?;
                rejected += 1;
                for issue in &issues {
                    val_repo.create_issue(run.id, change.id, issue.issue_type.clone(), issue.severity.clone(), &issue.message, issue.suggestion.as_deref())?;
                }
            } else if has_warning {
                val_repo.update_status(change.id, ProposedChangeStatus::PendingApproval)?;
                for issue in &issues {
                    val_repo.create_issue(run.id, change.id, issue.issue_type.clone(), issue.severity.clone(), &issue.message, issue.suggestion.as_deref())?;
                }
            } else {
                val_repo.update_status(change.id, ProposedChangeStatus::Approved)?;
                approved += 1;
            }
        }

        run.changes_validated = changes.len() as i32;
        run.changes_approved = approved;
        run.changes_rejected = rejected;
        run.status = ValidationStatus::Completed;
        run.completed_at = Some(Utc::now());
        val_repo.update_validation_run(&run)?;

        tracing::info!("Validation complete: {} validated, {} approved, {} rejected", changes.len(), approved, rejected);
        Ok(run)
    }

    fn validate_single_change(&self, change: &ProposedChange, entity_repo: &entity_repo::EntityRepo, _state_repo: &state_repo::StateRepo) -> Result<Vec<ValidationIssue>> {
        let mut issues = Vec::new();

        if entity_repo.get_by_id(change.target_entity_id)?.is_none() {
            issues.push(ValidationIssue {
                id: Uuid::new_v4(), validation_run_id: Uuid::nil(), proposed_change_id: change.id,
                issue_type: ValidationIssueType::EntityNotFound, severity: IssueSeverity::Critical,
                message: format!("Target entity {} not found", change.target_entity_id),
                suggestion: Some("Ensure the entity exists before proposing changes".to_string()),
                created_at: Utc::now(),
            });
            return Ok(issues);
        }

        match &change.change_type {
            ProposedChangeType::StateChange => {
                if change.payload.get("state_key").is_none() {
                    issues.push(ValidationIssue {
                        id: Uuid::new_v4(), validation_run_id: Uuid::nil(), proposed_change_id: change.id,
                        issue_type: ValidationIssueType::TypeMismatch, severity: IssueSeverity::Critical,
                        message: "StateChange payload missing 'state_key'".to_string(),
                        suggestion: Some("Include state_key in payload".to_string()),
                        created_at: Utc::now(),
                    });
                }
                if change.payload.get("new_value").is_none() {
                    issues.push(ValidationIssue {
                        id: Uuid::new_v4(), validation_run_id: Uuid::nil(), proposed_change_id: change.id,
                        issue_type: ValidationIssueType::TypeMismatch, severity: IssueSeverity::Critical,
                        message: "StateChange payload missing 'new_value'".to_string(),
                        suggestion: Some("Include new_value in payload".to_string()),
                        created_at: Utc::now(),
                    });
                }
            }
            _ => {}
        }

        Ok(issues)
    }

    /// 列出已批准的变更
    pub fn list_approved_changes(&self, project_id: Uuid, task_id: Uuid) -> Result<Vec<ProposedChange>> {
        let conn = self.db.conn();
        let mut stmt = conn.prepare(
            "SELECT id, project_id, task_id, change_type, target_entity_id, description, payload, status, created_at, resolved_at FROM proposed_change WHERE project_id = ? AND task_id = ? AND status = 'Approved' ORDER BY created_at",
        ).context("Failed to prepare")?;

        let mut rows = stmt.query([project_id.to_string(), task_id.to_string()]).context("Failed to query")?;

        let mut result = Vec::new();
        while let Some(row) = rows.next().context("Failed to iterate")? {
            let ct_str: String = row.get(3)?;
            let payload_str: String = row.get(6)?;
            let resolved_str: Option<String> = row.get::<_, Option<String>>(9).unwrap_or(None);
            result.push(ProposedChange {
                id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
                project_id: Uuid::parse_str(&row.get::<_, String>(1)?).unwrap(),
                task_id: Uuid::parse_str(&row.get::<_, String>(2)?).unwrap(),
                change_type: db::ser::parse_proposed_change_type(&ct_str),
                target_entity_id: Uuid::parse_str(&row.get::<_, String>(4)?).unwrap(),
                description: row.get(5)?,
                payload: serde_json::from_str(&payload_str).unwrap_or_default(),
                status: ProposedChangeStatus::Approved,
                created_at: db::time_utils::get_timestamp(&row, 8),
                resolved_at: resolved_str.and_then(|r| r.parse().ok()),
            });
        }
        Ok(result)
    }

    /// 批准并提交变更
    pub fn apply_approved_changes(&self, project_id: Uuid, task_id: Uuid) -> Result<Vec<StateChangeRecord>> {
        let val_repo = validation_repo::ValidationRepo::new(self.db);
        let state_repo = state_repo::StateRepo::new(self.db);
        let mut records = Vec::new();

        let approved_changes = self.list_approved_changes(project_id, task_id)?;

        for change in &approved_changes {
            match &change.change_type {
                ProposedChangeType::StateChange => {
                    let state_key = change.payload.get("state_key").and_then(|v| v.as_str()).unwrap_or("");
                    let new_value = change.payload.get("new_value").cloned().unwrap_or(serde_json::Value::Null);

                    let old_state = state_repo.get_current_state(change.target_entity_id, state_key)?;
                    let old_value = old_state.map(|s| s.state_value);

                    let record = state_repo.record_change(project_id, None, "STATE_CHANGE", change.target_entity_id, state_key, old_value, new_value.clone(), Some("validator"))?;
                    state_repo.upsert_state(project_id, change.target_entity_id, state_key, new_value)?;
                    records.push(record);

                    val_repo.update_status(change.id, ProposedChangeStatus::Applied)?;
                }
                _ => { tracing::warn!("Unsupported change type: {:?}", change.change_type); }
            }
        }

        tracing::info!("Applied {} changes", records.len());
        Ok(records)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_db() -> Database {
        let db = Database::open_in_memory().unwrap();
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let migrations_dir = format!("{}/../db/migrations", manifest_dir);
        db::migration::run_migrations(&db, &migrations_dir).unwrap();
        db
    }

    fn create_test_project(db: &Database) -> Uuid {
        let repo = db::repos::project_repo::ProjectRepo::new(db);
        repo.create("Test Novel", None).unwrap().id
    }

    fn create_test_world(db: &Database, project_id: Uuid) -> Uuid {
        let ws = super::super::world_service::WorldService::new(db);
        ws.ensure_main_world(project_id, "Test Novel").unwrap().id
    }

    #[test]
    fn test_validate_valid_change() {
        let db = setup_db();
        let project_id = create_test_project(&db);
        let world_id = create_test_world(&db, project_id);
        let world_service = super::super::world_service::WorldService::new(&db);
        let validator = Validator::new(&db);

        let entity = world_service.create_entity(project_id, world_id, "Character", "Lin Fan", None, None, serde_json::json!({})).unwrap();

        let gen_runtime = super::super::generation_runtime::GenerationRuntime::new(&db);
        let (name, template, stype) = super::super::generation_runtime::SkillTemplates::writer();
        gen_runtime.register_skill(name, None, stype, template, None, None, serde_json::json!({})).unwrap();
        let task = gen_runtime.create_task(project_id, "writer", None, serde_json::json!({})).unwrap();

        let val_repo = validation_repo::ValidationRepo::new(&db);
        let change = val_repo.create_proposed_change(project_id, task.id, ProposedChangeType::StateChange, entity.id, "Move to city", serde_json::json!({"state_key": "location", "new_value": "Black Stone City"})).unwrap();

        let run = validator.validate_changes(project_id, task.id, &[change]).unwrap();
        assert_eq!(run.changes_approved, 1);
        assert_eq!(run.changes_rejected, 0);
    }

    #[test]
    fn test_validate_invalid_change() {
        let db = setup_db();
        let project_id = create_test_project(&db);
        let validator = Validator::new(&db);
        let fake_id = Uuid::new_v4();

        let gen_runtime = super::super::generation_runtime::GenerationRuntime::new(&db);
        let (name, template, stype) = super::super::generation_runtime::SkillTemplates::writer();
        gen_runtime.register_skill(name, None, stype, template, None, None, serde_json::json!({})).unwrap();
        let task = gen_runtime.create_task(project_id, "writer", None, serde_json::json!({})).unwrap();

        let val_repo = validation_repo::ValidationRepo::new(&db);
        let change = val_repo.create_proposed_change(project_id, task.id, ProposedChangeType::StateChange, fake_id, "Invalid change", serde_json::json!({"state_key": "location", "new_value": "Somewhere"})).unwrap();

        let run = validator.validate_changes(project_id, task.id, &[change]).unwrap();
        assert_eq!(run.changes_approved, 0);
        assert_eq!(run.changes_rejected, 1);
    }

    #[test]
    fn test_apply_approved_changes() {
        let db = setup_db();
        let project_id = create_test_project(&db);
        let world_id = create_test_world(&db, project_id);
        let world_service = super::super::world_service::WorldService::new(&db);
        let validator = Validator::new(&db);

        let entity = world_service.create_entity(project_id, world_id, "Character", "Lin Fan", None, None, serde_json::json!({})).unwrap();

        let gen_runtime = super::super::generation_runtime::GenerationRuntime::new(&db);
        let (name, template, stype) = super::super::generation_runtime::SkillTemplates::writer();
        gen_runtime.register_skill(name, None, stype, template, None, None, serde_json::json!({})).unwrap();
        let task = gen_runtime.create_task(project_id, "writer", None, serde_json::json!({})).unwrap();

        let val_repo = validation_repo::ValidationRepo::new(&db);
        let change = val_repo.create_proposed_change(project_id, task.id, ProposedChangeType::StateChange, entity.id, "Move to city", serde_json::json!({"state_key": "location", "new_value": "Black Stone City"})).unwrap();

        let _run = validator.validate_changes(project_id, task.id, &[change]).unwrap();

        let approved = validator.list_approved_changes(project_id, task.id).unwrap();
        assert_eq!(approved.len(), 1);

        let records = validator.apply_approved_changes(project_id, task.id).unwrap();
        assert_eq!(records.len(), 1);

        let state = world_service.get_entity_state(entity.id, "location").unwrap().unwrap();
        assert_eq!(state.state_value, serde_json::json!("Black Stone City"));
    }
}
