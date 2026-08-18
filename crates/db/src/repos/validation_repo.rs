//! Validation Repository

use anyhow::{Context, Result};
use chrono::Utc;
use domain::{IssueSeverity, ProposedChange, ProposedChangeStatus, ProposedChangeType, ValidationIssue, ValidationIssueType, ValidationRun, ValidationStatus};
use uuid::Uuid;

use crate::connection::Database;
use crate::time_utils::get_timestamp;

pub struct ValidationRepo<'a> { db: &'a Database }

impl<'a> ValidationRepo<'a> {
    pub fn new(db: &'a Database) -> Self { Self { db } }

    pub fn create_proposed_change(&self, project_id: Uuid, task_id: Uuid, change_type: ProposedChangeType, target_entity_id: Uuid, description: &str, payload: serde_json::Value) -> Result<ProposedChange> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let ct_str = crate::ser::proposed_change_type_str(&change_type);
        let conn = self.db.conn();
        conn.execute("INSERT INTO proposed_change (id, project_id, task_id, change_type, target_entity_id, description, payload, status, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, 'Pending', ?)", [id.to_string(), project_id.to_string(), task_id.to_string(), ct_str, target_entity_id.to_string(), description.to_string(), payload.to_string(), now.to_string()]).context("Failed to create")?;
        Ok(ProposedChange { id, project_id, task_id, change_type, target_entity_id, description: description.to_string(), payload, status: ProposedChangeStatus::Draft, created_at: now, resolved_at: None })
    }

    pub fn update_status(&self, change_id: Uuid, status: ProposedChangeStatus) -> Result<()> {
        let conn = self.db.conn();
        let status_str = match status {
            ProposedChangeStatus::Draft => "Draft".to_string(),
            ProposedChangeStatus::Validating => "Validating".to_string(),
            ProposedChangeStatus::Valid => "Valid".to_string(),
            ProposedChangeStatus::Approved => "Approved".to_string(),
            ProposedChangeStatus::PendingApproval => "PendingApproval".to_string(),
            ProposedChangeStatus::Committed => "Committed".to_string(),
            ProposedChangeStatus::Applied => "Applied".to_string(),
            ProposedChangeStatus::Invalid => "Invalid".to_string(),
            ProposedChangeStatus::Rejected => "Rejected".to_string(),
            ProposedChangeStatus::Conflicted => "Conflicted".to_string(),
            ProposedChangeStatus::Expired => "Expired".to_string(),
        };
        // DuckDB stores UUID as INT128, need to cast properly
        let sql = "UPDATE proposed_change SET status = ?, resolved_at = ? WHERE id = ?";
        let rows_affected = conn.execute(sql, [status_str, Utc::now().to_string(), change_id.to_string()]).context("Failed to update")?;
        eprintln!("UPDATE result: {} rows affected for change {}", rows_affected, change_id);
        Ok(())
    }

    pub fn list_pending(&self, project_id: Uuid) -> Result<Vec<ProposedChange>> {
        let conn = self.db.conn();
        let mut stmt = conn.prepare("SELECT id, project_id, task_id, change_type, target_entity_id, description, payload, status, created_at, resolved_at FROM proposed_change WHERE project_id = ? AND status = 'Pending' ORDER BY created_at").context("Failed to prepare")?;
        let rows = stmt.query_map([project_id.to_string()], |row| {
            let resolved: Option<String> = row.get(9)?;
            Ok(ProposedChange {
                id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
                project_id: Uuid::parse_str(&row.get::<_, String>(1)?).unwrap(),
                task_id: Uuid::parse_str(&row.get::<_, String>(2)?).unwrap(),
                change_type: crate::ser::parse_proposed_change_type(&row.get::<_, String>(3)?),
                target_entity_id: Uuid::parse_str(&row.get::<_, String>(4)?).unwrap(),
                description: row.get(5)?,
                payload: serde_json::from_str(&row.get::<_, String>(6)?).unwrap_or_default(),
                status: crate::ser::parse_proposed_change_status("Pending"),
                created_at: get_timestamp(row, 8),
                resolved_at: resolved.and_then(|r| r.parse().ok()),
            })
        }).context("Failed to query")?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    pub fn create_validation_run(&self, project_id: Uuid, task_id: Uuid) -> Result<ValidationRun> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let conn = self.db.conn();
        conn.execute("INSERT INTO validation_run (id, project_id, task_id, status, started_at) VALUES (?, ?, ?, 'Running', ?)", [id.to_string(), project_id.to_string(), task_id.to_string(), now.to_string()]).context("Failed to create")?;
        Ok(ValidationRun { id, project_id, task_id, changes_validated: 0, changes_approved: 0, changes_rejected: 0, status: ValidationStatus::Running, started_at: now, completed_at: None })
    }

    pub fn update_validation_run(&self, run: &ValidationRun) -> Result<()> {
        let conn = self.db.conn();
        let status_str = crate::ser::validation_status_str(&run.status);
        conn.execute("UPDATE validation_run SET changes_validated = ?, changes_approved = ?, changes_rejected = ?, status = ?, completed_at = ? WHERE id = ?", [run.changes_validated.to_string(), run.changes_approved.to_string(), run.changes_rejected.to_string(), status_str, run.completed_at.map(|t| t.to_string()).unwrap_or_default(), run.id.to_string()]).context("Failed to update")?;
        Ok(())
    }

    pub fn create_issue(&self, validation_run_id: Uuid, proposed_change_id: Uuid, issue_type: ValidationIssueType, severity: IssueSeverity, message: &str, suggestion: Option<&str>) -> Result<ValidationIssue> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let it_str = crate::ser::validation_issue_type_str(&issue_type);
        let sev_str = crate::ser::issue_severity_str(&severity);
        let conn = self.db.conn();
        conn.execute("INSERT INTO validation_issue (id, validation_run_id, proposed_change_id, issue_type, severity, message, suggestion, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)", [id.to_string(), validation_run_id.to_string(), proposed_change_id.to_string(), it_str, sev_str, message.to_string(), suggestion.unwrap_or("").to_string(), now.to_string()]).context("Failed to create")?;
        Ok(ValidationIssue { id, validation_run_id, proposed_change_id, issue_type, severity, message: message.to_string(), suggestion: suggestion.map(|s| s.to_string()), created_at: now })
    }

    pub fn list_issues_by_run(&self, validation_run_id: Uuid) -> Result<Vec<ValidationIssue>> {
        let conn = self.db.conn();
        let mut stmt = conn.prepare("SELECT id, validation_run_id, proposed_change_id, issue_type, severity, message, suggestion, created_at FROM validation_issue WHERE validation_run_id = ? ORDER BY created_at").context("Failed to prepare")?;
        let rows = stmt.query_map([validation_run_id.to_string()], |row| {
            Ok(ValidationIssue {
                id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
                validation_run_id: Uuid::parse_str(&row.get::<_, String>(1)?).unwrap(),
                proposed_change_id: Uuid::parse_str(&row.get::<_, String>(2)?).unwrap(),
                issue_type: crate::ser::parse_validation_issue_type(&row.get::<_, String>(3)?),
                severity: crate::ser::parse_issue_severity(&row.get::<_, String>(4)?),
                message: row.get(5)?,
                suggestion: row.get(6)?,
                created_at: get_timestamp(row, 7),
            })
        }).context("Failed to query")?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }
}
