//! Approval Repository - CRUD operations for ApprovalRecord

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use domain::{ApprovalRecord, ApprovalStatus, ApprovalTargetType};
use sqlx::PgPool;
use uuid::Uuid;

pub struct ApprovalRepo {
    pool: PgPool,
}

impl ApprovalRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 创建审批记录
    pub async fn create(
        &self,
        project_id: Uuid,
        target_type: ApprovalTargetType,
        target_id: Uuid,
        proposed_by: &str,
        proposal_content: serde_json::Value,
    ) -> Result<ApprovalRecord> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let tt_str = match &target_type {
            ApprovalTargetType::World => "World",
            ApprovalTargetType::Entity => "Entity",
            ApprovalTargetType::Volume => "Volume",
            ApprovalTargetType::Arc => "Arc",
            ApprovalTargetType::Scene => "Scene",
            ApprovalTargetType::Storyline => "Storyline",
            ApprovalTargetType::Fact => "Fact",
            ApprovalTargetType::Custom(s) => s.as_str(),
        };

        sqlx::query(
            "INSERT INTO approval_record (id, project_id, target_type, target_id, proposed_by, proposal_content, status, created_at) \
             VALUES ($1, $2, $3, $4, $5, $6, 'Pending', $7)",
        )
        .bind(id)
        .bind(project_id)
        .bind(tt_str)
        .bind(target_id)
        .bind(proposed_by)
        .bind(&proposal_content)
        .bind(now)
        .execute(&self.pool)
        .await
        .context("Failed to create approval record")?;

        Ok(ApprovalRecord {
            id,
            project_id,
            target_type,
            target_id,
            proposed_by: proposed_by.to_string(),
            proposal_content,
            status: ApprovalStatus::Pending,
            reviewer_id: None,
            reviewer_comment: None,
            created_at: now,
            reviewed_at: None,
        })
    }

    /// 审批记录
    pub async fn approve(&self, id: Uuid, reviewer_id: &str, comment: Option<&str>) -> Result<()> {
        sqlx::query(
            "UPDATE approval_record SET status = 'Approved', reviewer_id = $1, reviewer_comment = $2, reviewed_at = NOW() WHERE id = $3",
        )
        .bind(reviewer_id)
        .bind(comment.unwrap_or(""))
        .bind(id)
        .execute(&self.pool)
        .await
        .context("Failed to approve record")?;
        Ok(())
    }

    /// 拒绝记录
    pub async fn reject(&self, id: Uuid, reviewer_id: &str, comment: Option<&str>) -> Result<()> {
        sqlx::query(
            "UPDATE approval_record SET status = 'Rejected', reviewer_id = $1, reviewer_comment = $2, reviewed_at = NOW() WHERE id = $3",
        )
        .bind(reviewer_id)
        .bind(comment.unwrap_or(""))
        .bind(id)
        .execute(&self.pool)
        .await
        .context("Failed to reject record")?;
        Ok(())
    }

    /// 获取待审批记录
    pub async fn list_pending(&self, project_id: Uuid) -> Result<Vec<ApprovalRecord>> {
        let rows = sqlx::query_as::<_, ApprovalRow>(
            "SELECT id, project_id, target_type, target_id, proposed_by, proposal_content, status, reviewer_id, reviewer_comment, created_at, reviewed_at \
             FROM approval_record WHERE project_id = $1 AND status = 'Pending' ORDER BY created_at",
        )
        .bind(project_id)
        .fetch_all(&self.pool)
        .await
        .context("Failed to query approvals")?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }
}

#[derive(sqlx::FromRow)]
struct ApprovalRow {
    id: Uuid,
    project_id: Uuid,
    target_type: String,
    target_id: Uuid,
    proposed_by: String,
    proposal_content: Option<serde_json::Value>,
    status: String,
    reviewer_id: Option<String>,
    reviewer_comment: Option<String>,
    created_at: DateTime<Utc>,
    reviewed_at: Option<DateTime<Utc>>,
}

impl From<ApprovalRow> for ApprovalRecord {
    fn from(r: ApprovalRow) -> Self {
        let target_type = match r.target_type.as_str() {
            "World" => ApprovalTargetType::World,
            "Entity" => ApprovalTargetType::Entity,
            "Volume" => ApprovalTargetType::Volume,
            "Arc" => ApprovalTargetType::Arc,
            "Scene" => ApprovalTargetType::Scene,
            "Storyline" => ApprovalTargetType::Storyline,
            "Fact" => ApprovalTargetType::Fact,
            s => ApprovalTargetType::Custom(s.to_string()),
        };
        ApprovalRecord {
            id: r.id,
            project_id: r.project_id,
            target_type,
            target_id: r.target_id,
            proposed_by: r.proposed_by,
            proposal_content: r.proposal_content.unwrap_or_default(),
            status: ApprovalStatus::Pending,
            reviewer_id: r.reviewer_id,
            reviewer_comment: r.reviewer_comment,
            created_at: r.created_at,
            reviewed_at: r.reviewed_at,
        }
    }
}
