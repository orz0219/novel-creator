//! Proposal Service - 提案管理的业务逻辑层
//!
//! 负责提案的创建、验证、批准、拒绝、提交。
//! 所有状态转换必须通过 ProposedChangeStatus.can_transition_to() 验证。

use anyhow::{Context, Result};
use chrono::Utc;
use domain::*;
use sqlx::PgPool;
use uuid::Uuid;

/// Proposal Service - 提案管理服务
pub struct ProposalService {
    pool: PgPool,
}

impl ProposalService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 列出项目的所有提案
    pub async fn list_proposals(&self, project_id: Uuid) -> Result<Vec<ProposedChange>> {
        let rows = sqlx::query_as::<_, ProposedChangeRow>(
            "SELECT id, project_id, task_id, change_type, target_entity_id, description, payload, status, created_at, resolved_at \
             FROM proposed_change WHERE project_id = $1 ORDER BY created_at DESC"
        )
        .bind(project_id)
        .fetch_all(&self.pool)
        .await
        .context("Failed to list proposals")?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    /// 获取单个提案
    pub async fn get_proposal(&self, id: Uuid) -> Result<Option<ProposedChange>> {
        let row = sqlx::query_as::<_, ProposedChangeRow>(
            "SELECT id, project_id, task_id, change_type, target_entity_id, description, payload, status, created_at, resolved_at \
             FROM proposed_change WHERE id = $1"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to get proposal")?;

        Ok(row.map(|r| r.into()))
    }

    /// 批准提案 - 验证状态转换后更新
    pub async fn approve_proposal(&self, id: Uuid) -> Result<ProposedChange> {
        // 获取当前状态
        let current = self.get_proposal(id).await?
            .ok_or_else(|| anyhow::anyhow!("Proposal not found"))?;

        // 验证状态转换
        if !current.status.can_transition_to(&ProposedChangeStatus::Approved) {
            return Err(anyhow::anyhow!(
                "Invalid state transition: {} -> {}",
                current.status.description(),
                ProposedChangeStatus::Approved.description()
            ));
        }

        // CAS 更新
        let status_str = db::ser::proposed_change_status_str(&current.status);
        let result = sqlx::query(
            "UPDATE proposed_change SET status = 'Approved', resolved_at = NOW() WHERE id = $1 AND status = $2"
        )
        .bind(id)
        .bind(&status_str)
        .execute(&self.pool)
        .await
        .context("Failed to approve proposal")?;

        if result.rows_affected() == 0 {
            return Err(anyhow::anyhow!("Concurrent modification: proposal status changed during update"));
        }

        // 返回更新后的提案
        self.get_proposal(id).await?
            .ok_or_else(|| anyhow::anyhow!("Proposal disappeared after update"))
    }

    /// 拒绝提案 - 验证状态转换后更新
    pub async fn reject_proposal(&self, id: Uuid) -> Result<ProposedChange> {
        let current = self.get_proposal(id).await?
            .ok_or_else(|| anyhow::anyhow!("Proposal not found"))?;

        if !current.status.can_transition_to(&ProposedChangeStatus::Rejected) {
            return Err(anyhow::anyhow!(
                "Invalid state transition: {} -> {}",
                current.status.description(),
                ProposedChangeStatus::Rejected.description()
            ));
        }

        let status_str = db::ser::proposed_change_status_str(&current.status);
        let result = sqlx::query(
            "UPDATE proposed_change SET status = 'Rejected', resolved_at = NOW() WHERE id = $1 AND status = $2"
        )
        .bind(id)
        .bind(&status_str)
        .execute(&self.pool)
        .await
        .context("Failed to reject proposal")?;

        if result.rows_affected() == 0 {
            return Err(anyhow::anyhow!("Concurrent modification: proposal status changed during update"));
        }

        self.get_proposal(id).await?
            .ok_or_else(|| anyhow::anyhow!("Proposal disappeared after update"))
    }

    /// 创建提案
    pub async fn create_proposal(
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
        let ct_str = db::ser::proposed_change_type_str(&change_type);

        sqlx::query(
            "INSERT INTO proposed_change (id, project_id, task_id, change_type, target_entity_id, description, payload, status, created_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, 'Draft', $8)"
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
        .context("Failed to create proposal")?;

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
    created_at: chrono::DateTime<Utc>,
    resolved_at: Option<chrono::DateTime<Utc>>,
}

impl From<ProposedChangeRow> for ProposedChange {
    fn from(r: ProposedChangeRow) -> Self {
        ProposedChange {
            id: r.id,
            project_id: r.project_id,
            task_id: r.task_id,
            change_type: db::ser::parse_proposed_change_type(&r.change_type),
            target_entity_id: r.target_entity_id,
            description: r.description,
            payload: r.payload.unwrap_or_default(),
            status: db::ser::parse_proposed_change_status(&r.status),
            created_at: r.created_at,
            resolved_at: r.resolved_at,
        }
    }
}
