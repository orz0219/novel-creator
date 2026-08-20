//! Validation Repository
//!
//! Provides both pool-based and transaction-aware methods.
//! Transaction-aware methods (suffixed _tx) accept a &mut PgConnection
//! and should be used when multiple operations must be atomic.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use domain::{
    IssueSeverity, ProposedChange, ProposedChangeStatus, ProposedChangeType, ValidationIssue,
    ValidationIssueType, ValidationRun, ValidationStatus,
};
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

use crate::ser;

pub struct ValidationRepo {
    pool: PgPool,
}

impl ValidationRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create_proposed_change(
        &self,
        project_id: Uuid,
        task_id: Uuid,
        change_type: ProposedChangeType,
        target_entity_id: Uuid,
        description: &str,
        payload: serde_json::Value,
    ) -> Result<ProposedChange> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let ct_str = ser::proposed_change_type_str(&change_type);

        sqlx::query(
            "INSERT INTO proposed_change (id, project_id, task_id, change_type, target_entity_id, description, payload, status, created_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, 'Pending', $8)",
        )
        .bind(id)
        .bind(project_id)
        .bind(task_id)
        .bind(&ct_str)
        .bind(target_entity_id)
        .bind(description)
        .bind(&payload)
        .bind(now)
        .execute(&self.pool)
        .await
        .context("Failed to create proposed change")?;

        Ok(ProposedChange {
            id,
            project_id,
            task_id,
            change_type,
            target_entity_id,
            description: description.to_string(),
            payload,
            status: ProposedChangeStatus::Draft,
            created_at: now,
            resolved_at: None,
        })
    }

    /// Update status with state machine validation.
    ///
    /// P2-7: Enforces valid state transitions:
    /// - Draft -> PendingApproval, Approved, Rejected
    /// - PendingApproval -> Approved, Rejected
    /// - Approved -> Applied, Rejected
    /// - Applied, Rejected, Invalid, Expired -> no transitions (terminal states)
    ///
    /// For production use, prefer update_status_with_guard_tx which also does CAS.
    pub async fn update_status(&self, change_id: Uuid, status: ProposedChangeStatus) -> Result<()> {
        Self::update_status_tx(&mut *self.pool.acquire().await.context("Failed to acquire connection")?, change_id, status).await
    }

    /// Transaction-aware update_status with state machine validation.
    ///
    /// P2-7: Validates that the transition is allowed before updating.
    pub async fn update_status_tx(
        conn: &mut PgConnection,
        change_id: Uuid,
        status: ProposedChangeStatus,
    ) -> Result<()> {
        // P2-7: 验证状态转换是否合法
        // 先获取当前状态
        let current: Option<String> = sqlx::query_scalar(
            "SELECT status FROM proposed_change WHERE id = $1"
        )
        .bind(change_id)
        .fetch_optional(&mut *conn)
        .await
        .context("Failed to query current status")?;

        if let Some(current_str) = current {
            let current_status = ser::parse_proposed_change_status(&current_str);
            if !Self::is_valid_transition(&current_status, &status) {
                return Err(anyhow::anyhow!(
                    "Invalid status transition: {:?} -> {:?}. This transition is not allowed by the state machine.",
                    current_status, status
                ));
            }
        }

        let status_str = ser::proposed_change_status_str(&status);

        // P2-8: 只有终态才设置 resolved_at
        let resolved_at = if Self::is_terminal_status(&status) {
            Some(Utc::now())
        } else {
            None
        };

        sqlx::query(
            "UPDATE proposed_change SET status = $1, resolved_at = $2 WHERE id = $3",
        )
        .bind(&status_str)
        .bind(resolved_at)
        .bind(change_id)
        .execute(&mut *conn)
        .await
        .context("Failed to update proposed change")?;
        Ok(())
    }

    /// Check if a status transition is valid
    fn is_valid_transition(from: &ProposedChangeStatus, to: &ProposedChangeStatus) -> bool {
        match (from, to) {
            // Draft 可以转到 PendingApproval, Approved, Rejected
            (ProposedChangeStatus::Draft, ProposedChangeStatus::PendingApproval) => true,
            (ProposedChangeStatus::Draft, ProposedChangeStatus::Approved) => true,
            (ProposedChangeStatus::Draft, ProposedChangeStatus::Rejected) => true,
            // PendingApproval 可以转到 Approved, Rejected
            (ProposedChangeStatus::PendingApproval, ProposedChangeStatus::Approved) => true,
            (ProposedChangeStatus::PendingApproval, ProposedChangeStatus::Rejected) => true,
            // Approved 可以转到 Applied, Rejected
            (ProposedChangeStatus::Approved, ProposedChangeStatus::Applied) => true,
            (ProposedChangeStatus::Approved, ProposedChangeStatus::Rejected) => true,
            // 终态不能转换
            (ProposedChangeStatus::Applied, _) => false,
            (ProposedChangeStatus::Rejected, _) => false,
            (ProposedChangeStatus::Invalid, _) => false,
            (ProposedChangeStatus::Expired, _) => false,
            // 其他情况不允许
            _ => false,
        }
    }

    /// Check if a status is terminal (no further transitions allowed)
    fn is_terminal_status(status: &ProposedChangeStatus) -> bool {
        matches!(
            status,
            ProposedChangeStatus::Applied
                | ProposedChangeStatus::Rejected
                | ProposedChangeStatus::Invalid
                | ProposedChangeStatus::Expired
        )
    }

    /// CAS-guarded status transition. Only updates if current status == from_status.
    ///
    /// P2-8: 只有终态 (Applied, Rejected, Invalid, Expired) 才设置 resolved_at。
    /// 非终态转换时 resolved_at 保持 NULL。
    ///
    /// Returns rows_affected: 1 if transition succeeded, 0 if status mismatch.
    pub async fn update_status_with_guard_tx(
        conn: &mut PgConnection,
        change_id: Uuid,
        to_status: ProposedChangeStatus,
        from_status: ProposedChangeStatus,
    ) -> Result<u64> {
        let to_str = ser::proposed_change_status_str(&to_status);
        let from_str = ser::proposed_change_status_str(&from_status);

        // P2-8: 只有终态才设置 resolved_at
        let resolved_at = if Self::is_terminal_status(&to_status) {
            Some(Utc::now())
        } else {
            None
        };

        let result = sqlx::query(
            "UPDATE proposed_change SET status = $1, resolved_at = $2 WHERE id = $3 AND status = $4",
        )
        .bind(&to_str)
        .bind(resolved_at)
        .bind(change_id)
        .bind(&from_str)
        .execute(&mut *conn)
        .await
        .context("Failed to update proposed change with guard")?;
        Ok(result.rows_affected())
    }

    /// P1-2: 获取 ProposedChange 的权威版本（从数据库读取）
    /// StateCommitter 应该使用此方法重新加载 proposal，而不是依赖传入的快照。
    pub async fn get_proposed_change_by_id(&self, change_id: Uuid) -> Result<Option<ProposedChange>> {
        let row = sqlx::query_as::<_, ProposedChangeRow>(
            "SELECT id, project_id, task_id, change_type, target_entity_id, description, payload, status, created_at, resolved_at              FROM proposed_change WHERE id = $1",
        )
        .bind(change_id)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to query proposed change")?;

        Ok(row.map(|r| r.into()))
    }

    /// Transaction-aware version of get_proposed_change_by_id
    pub async fn get_proposed_change_by_id_tx(
        conn: &mut PgConnection,
        change_id: Uuid,
    ) -> Result<Option<ProposedChange>> {
        let row = sqlx::query_as::<_, ProposedChangeRow>(
            "SELECT id, project_id, task_id, change_type, target_entity_id, description, payload, status, created_at, resolved_at              FROM proposed_change WHERE id = $1",
        )
        .bind(change_id)
        .fetch_optional(&mut *conn)
        .await
        .context("Failed to query proposed change")?;

        Ok(row.map(|r| r.into()))
    }

    /// Transaction-aware version with FOR UPDATE row lock.
    /// Prevents concurrent modification of the same proposal during commit.
    pub async fn get_proposed_change_by_id_for_update_tx(
        conn: &mut PgConnection,
        change_id: Uuid,
    ) -> Result<Option<ProposedChange>> {
        let row = sqlx::query_as::<_, ProposedChangeRow>(
            "SELECT id, project_id, task_id, change_type, target_entity_id, description, payload, status, created_at, resolved_at FROM proposed_change WHERE id = $1 FOR UPDATE",
        )
        .bind(change_id)
        .fetch_optional(&mut *conn)
        .await
        .context("Failed to query proposed change with FOR UPDATE")?;

        Ok(row.map(|r| r.into()))
    }

    pub async fn list_pending(&self, project_id: Uuid) -> Result<Vec<ProposedChange>> {
        let rows = sqlx::query_as::<_, ProposedChangeRow>(
            "SELECT id, project_id, task_id, change_type, target_entity_id, description, payload, status, created_at, resolved_at \
             FROM proposed_change WHERE project_id = $1 AND status = 'Pending' ORDER BY created_at",
        )
        .bind(project_id)
        .fetch_all(&self.pool)
        .await
        .context("Failed to query proposed changes")?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    pub async fn create_validation_run(
        &self,
        project_id: Uuid,
        task_id: Uuid,
    ) -> Result<ValidationRun> {
        let id = Uuid::new_v4();
        let now = Utc::now();

        sqlx::query(
            "INSERT INTO validation_run (id, project_id, task_id, status, started_at) \
             VALUES ($1, $2, $3, 'Running', $4)",
        )
        .bind(id)
        .bind(project_id)
        .bind(task_id)
        .bind(now)
        .execute(&self.pool)
        .await
        .context("Failed to create validation run")?;

        Ok(ValidationRun {
            id,
            project_id,
            task_id,
            changes_validated: 0,
            changes_approved: 0,
            changes_rejected: 0,
            status: ValidationStatus::Running,
            started_at: now,
            completed_at: None,
        })
    }

    pub async fn update_validation_run(&self, run: &ValidationRun) -> Result<()> {
        let status_str = ser::validation_status_str(&run.status);

        sqlx::query(
            "UPDATE validation_run SET changes_validated = $1, changes_approved = $2, changes_rejected = $3, status = $4, completed_at = $5 WHERE id = $6",
        )
        .bind(run.changes_validated)
        .bind(run.changes_approved)
        .bind(run.changes_rejected)
        .bind(&status_str)
        .bind(run.completed_at)
        .bind(run.id)
        .execute(&self.pool)
        .await
        .context("Failed to update validation run")?;
        Ok(())
    }

    pub async fn create_issue(
        &self,
        validation_run_id: Uuid,
        proposed_change_id: Uuid,
        issue_type: ValidationIssueType,
        severity: IssueSeverity,
        message: &str,
        suggestion: Option<&str>,
    ) -> Result<ValidationIssue> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let it_str = ser::validation_issue_type_str(&issue_type);
        let sev_str = ser::issue_severity_str(&severity);

        sqlx::query(
            "INSERT INTO validation_issue (id, validation_run_id, proposed_change_id, issue_type, severity, message, suggestion, created_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(id)
        .bind(validation_run_id)
        .bind(proposed_change_id)
        .bind(&it_str)
        .bind(&sev_str)
        .bind(message)
        .bind(suggestion.unwrap_or(""))
        .bind(now)
        .execute(&self.pool)
        .await
        .context("Failed to create validation issue")?;

        Ok(ValidationIssue {
            id,
            validation_run_id,
            proposed_change_id,
            issue_type,
            severity,
            message: message.to_string(),
            suggestion: suggestion.map(|s| s.to_string()),
            created_at: now,
        })
    }

    pub async fn list_issues_by_run(
        &self,
        validation_run_id: Uuid,
    ) -> Result<Vec<ValidationIssue>> {
        let rows = sqlx::query_as::<_, ValidationIssueRow>(
            "SELECT id, validation_run_id, proposed_change_id, issue_type, severity, message, suggestion, created_at \
             FROM validation_issue WHERE validation_run_id = $1 ORDER BY created_at",
        )
        .bind(validation_run_id)
        .fetch_all(&self.pool)
        .await
        .context("Failed to query validation issues")?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
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

impl From<ProposedChangeRow> for ProposedChange {
    fn from(r: ProposedChangeRow) -> Self {
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
}

#[derive(sqlx::FromRow)]
struct ValidationIssueRow {
    id: Uuid,
    validation_run_id: Uuid,
    proposed_change_id: Uuid,
    issue_type: String,
    severity: String,
    message: String,
    suggestion: Option<String>,
    created_at: DateTime<Utc>,
}

impl From<ValidationIssueRow> for ValidationIssue {
    fn from(r: ValidationIssueRow) -> Self {
        ValidationIssue {
            id: r.id,
            validation_run_id: r.validation_run_id,
            proposed_change_id: r.proposed_change_id,
            issue_type: ser::parse_validation_issue_type(&r.issue_type),
            severity: ser::parse_issue_severity(&r.severity),
            message: r.message,
            suggestion: r.suggestion,
            created_at: r.created_at,
        }
    }
}
