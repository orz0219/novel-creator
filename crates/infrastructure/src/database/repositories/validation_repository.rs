//! DuckDB implementation of Validation Repository

use anyhow::Result;
use domain::validation::{ProposedChange, ProposedChangeStatus, ValidationRun, ValidationIssue};
use duckdb::Connection;
use uuid::Uuid;

/// DuckDB implementation of validation repository
pub struct DuckDbValidationRepository {
    conn: Connection,
}

impl DuckDbValidationRepository {
    pub fn new(conn: Connection) -> Self {
        Self { conn }
    }

    /// Get proposed change by ID
    pub fn get_proposed_change(&self, id: Uuid) -> Result<Option<ProposedChange>> {
        let result = self.conn.query_row(
            "SELECT id, project_id, task_id, change_type, target_entity_id, description, payload, status, created_at, resolved_at FROM proposed_change WHERE id = ?",
            [id.to_string()],
            |row| {
                Ok(ProposedChange {
                    id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
                    project_id: Uuid::parse_str(&row.get::<_, String>(1)?).unwrap(),
                    task_id: Uuid::parse_str(&row.get::<_, String>(2)?).unwrap(),
                    change_type: match row.get::<_, String>(3)?.as_str() {
                        "StateChange" => domain::validation::ProposedChangeType::StateChange,
                        "EntityCreate" => domain::validation::ProposedChangeType::EntityCreate,
                        "EntityUpdate" => domain::validation::ProposedChangeType::EntityUpdate,
                        _ => domain::validation::ProposedChangeType::Custom("unknown".to_string()),
                    },
                    target_entity_id: Uuid::parse_str(&row.get::<_, String>(4)?).unwrap(),
                    description: row.get::<_, String>(5)?,
                    payload: serde_json::from_str(&row.get::<_, String>(6)?).unwrap_or_default(),
                    status: match row.get::<_, String>(7)?.as_str() {
                        "Approved" => ProposedChangeStatus::Approved,
                        "PendingApproval" => ProposedChangeStatus::PendingApproval,
                        "Rejected" => ProposedChangeStatus::Rejected,
                        "Applied" => ProposedChangeStatus::Applied,
                        "Committed" => ProposedChangeStatus::Committed,
                        "Invalid" => ProposedChangeStatus::Invalid,
                        "Conflicted" => ProposedChangeStatus::Conflicted,
                        "Expired" => ProposedChangeStatus::Expired,
                        _ => ProposedChangeStatus::Draft,
                    },
                    created_at: chrono::Utc::now(),
                    resolved_at: None,
                })
            },
        );

        match result {
            Ok(change) => Ok(Some(change)),
            Err(duckdb::Error::QueryFailed(_)) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Create a proposed change
    pub fn create_proposed_change(&self, change: &ProposedChange) -> Result<ProposedChange> {
        self.conn.execute(
            "INSERT INTO proposed_change (id, project_id, task_id, change_type, target_entity_id, description, payload, status, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
            [
                change.id.to_string(),
                change.project_id.to_string(),
                change.task_id.to_string(),
                format!("{:?}", change.change_type),
                change.target_entity_id.to_string(),
                change.description.clone(),
                serde_json::to_string(&change.payload)?,
                format!("{:?}", change.status),
                change.created_at.to_rfc3339(),
            ],
        )?;
        Ok(change.clone())
    }

    /// Update proposed change status
    pub fn update_status(&self, id: Uuid, status: ProposedChangeStatus) -> Result<()> {
        self.conn.execute(
            "UPDATE proposed_change SET status = ? WHERE id = ?",
            [format!("{:?}", status), id.to_string()],
        )?;
        Ok(())
    }

    /// Get validation run by ID
    pub fn get_validation_run(&self, id: Uuid) -> Result<Option<ValidationRun>> {
        let result = self.conn.query_row(
            "SELECT id, project_id, task_id, changes_validated, changes_approved, changes_rejected, status, started_at, completed_at FROM validation_run WHERE id = ?",
            [id.to_string()],
            |row| {
                Ok(ValidationRun {
                    id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
                    project_id: Uuid::parse_str(&row.get::<_, String>(1)?).unwrap(),
                    task_id: Uuid::parse_str(&row.get::<_, String>(2)?).unwrap(),
                    changes_validated: row.get::<_, i32>(3)?,
                    changes_approved: row.get::<_, i32>(4)?,
                    changes_rejected: row.get::<_, i32>(5)?,
                    status: match row.get::<_, String>(6)?.as_str() {
                        "Completed" => domain::validation::ValidationStatus::Completed,
                        "Failed" => domain::validation::ValidationStatus::Failed,
                        _ => domain::validation::ValidationStatus::Running,
                    },
                    started_at: chrono::Utc::now(),
                    completed_at: None,
                })
            },
        );

        match result {
            Ok(run) => Ok(Some(run)),
            Err(duckdb::Error::QueryFailed(_)) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Create validation run
    pub fn create_validation_run(&self, run: &ValidationRun) -> Result<ValidationRun> {
        self.conn.execute(
            "INSERT INTO validation_run (id, project_id, task_id, changes_validated, changes_approved, changes_rejected, status, started_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            [
                run.id.to_string(),
                run.project_id.to_string(),
                run.task_id.to_string(),
                run.changes_validated.to_string(),
                run.changes_approved.to_string(),
                run.changes_rejected.to_string(),
                format!("{:?}", run.status),
                run.started_at.to_rfc3339(),
            ],
        )?;
        Ok(run.clone())
    }

    /// Create validation issue
    pub fn create_validation_issue(&self, issue: &ValidationIssue) -> Result<ValidationIssue> {
        self.conn.execute(
            "INSERT INTO validation_issue (id, validation_run_id, proposed_change_id, issue_type, severity, message, suggestion, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            [
                issue.id.to_string(),
                issue.validation_run_id.to_string(),
                issue.proposed_change_id.to_string(),
                format!("{:?}", issue.issue_type),
                format!("{:?}", issue.severity),
                issue.message.clone(),
                issue.suggestion.clone().unwrap_or_default(),
                issue.created_at.to_rfc3339(),
            ],
        )?;
        Ok(issue.clone())
    }

    /// Get validation issues for a run
    pub fn get_issues_by_run(&self, run_id: Uuid) -> Result<Vec<ValidationIssue>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, validation_run_id, proposed_change_id, issue_type, severity, message, suggestion, created_at FROM validation_issue WHERE validation_run_id = ?",
        )?;

        let rows = stmt.query_map([run_id.to_string()], |row| {
            Ok(ValidationIssue {
                id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
                validation_run_id: Uuid::parse_str(&row.get::<_, String>(1)?).unwrap(),
                proposed_change_id: Uuid::parse_str(&row.get::<_, String>(2)?).unwrap(),
                issue_type: match row.get::<_, String>(3)?.as_str() {
                    "Contradiction" => domain::validation::ValidationIssueType::Contradiction,
                    "EntityNotFound" => domain::validation::ValidationIssueType::EntityNotFound,
                    "RuleViolation" => domain::validation::ValidationIssueType::RuleViolation,
                    _ => domain::validation::ValidationIssueType::Custom("unknown".to_string()),
                },
                severity: match row.get::<_, String>(4)?.as_str() {
                    "Critical" => domain::validation::IssueSeverity::Critical,
                    "Warning" => domain::validation::IssueSeverity::Warning,
                    _ => domain::validation::IssueSeverity::Info,
                },
                message: row.get::<_, String>(5)?,
                suggestion: row.get::<_, Option<String>>(6)?,
                created_at: chrono::Utc::now(),
            })
        })?;

        let mut issues = Vec::new();
        for row in rows {
            issues.push(row?);
        }

        Ok(issues)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::DatabasePool;

    #[test]
    fn test_proposed_change_crud() {
        let pool = DatabasePool::new_in_memory(1).unwrap();
        let conn = pool.get().unwrap();

        // Create table
        conn.execute_batch(
            "CREATE TABLE proposed_change (
                id VARCHAR PRIMARY KEY,
                project_id VARCHAR NOT NULL,
                task_id VARCHAR NOT NULL,
                change_type VARCHAR NOT NULL,
                target_entity_id VARCHAR NOT NULL,
                description TEXT NOT NULL,
                payload JSON NOT NULL,
                status VARCHAR NOT NULL DEFAULT 'Pending',
                created_at TIMESTAMP,
                resolved_at TIMESTAMP
            )"
        ).unwrap();

        let repo = DuckDbValidationRepository::new(conn);

        // Create proposed change
        let change = ProposedChange {
            id: Uuid::new_v4(),
            project_id: Uuid::new_v4(),
            task_id: Uuid::new_v4(),
            change_type: domain::validation::ProposedChangeType::StateChange,
            target_entity_id: Uuid::new_v4(),
            description: "Update character status".to_string(),
            payload: serde_json::json!({"field": "status", "value": "active"}),
            status: ProposedChangeStatus::Draft,
            created_at: chrono::Utc::now(),
            resolved_at: None,
        };

        let created = repo.create_proposed_change(&change).unwrap();
        assert_eq!(created.description, "Update character status");

        // Get proposed change
        let retrieved = repo.get_proposed_change(created.id).unwrap().unwrap();
        assert_eq!(retrieved.description, "Update character status");

        // Update status
        repo.update_status(created.id, ProposedChangeStatus::Approved).unwrap();

        // Verify update
        let updated = repo.get_proposed_change(created.id).unwrap().unwrap();
        assert_eq!(updated.status, ProposedChangeStatus::Approved);
    }
}
