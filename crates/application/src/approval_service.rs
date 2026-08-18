//! Approval Service - Human Approval Gate 业务逻辑

use anyhow::Result;
use db::repos::approval_repo::ApprovalRepo;
use domain::*;
use sqlx::PgPool;
use uuid::Uuid;

pub struct ApprovalService {
    pool: PgPool,
}

impl ApprovalService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 提交审批请求
    pub async fn submit_for_approval(
        &self,
        project_id: Uuid,
        target_type: ApprovalTargetType,
        target_id: Uuid,
        proposed_by: &str,
        content: serde_json::Value,
    ) -> Result<ApprovalRecord> {
        let repo = ApprovalRepo::new(self.pool.clone());
        repo.create(project_id, target_type, target_id, proposed_by, content).await
    }

    /// 批准
    pub async fn approve(&self, record_id: Uuid, reviewer_id: &str, comment: Option<&str>) -> Result<()> {
        let repo = ApprovalRepo::new(self.pool.clone());
        repo.approve(record_id, reviewer_id, comment).await
    }

    /// 拒绝
    pub async fn reject(&self, record_id: Uuid, reviewer_id: &str, comment: Option<&str>) -> Result<()> {
        let repo = ApprovalRepo::new(self.pool.clone());
        repo.reject(record_id, reviewer_id, comment).await
    }

    /// 获取待审批记录
    pub async fn get_pending(&self, project_id: Uuid) -> Result<Vec<ApprovalRecord>> {
        let repo = ApprovalRepo::new(self.pool.clone());
        repo.list_pending(project_id).await
    }
}
