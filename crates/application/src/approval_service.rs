//! Approval Service - Human Approval Gate 业务逻辑
//!
//! 通过 ApprovalRepositoryPort 访问数据，不直接依赖 db / sqlx。

use anyhow::Result;
use domain::approval::{ApprovalRecord, ApprovalTargetType};
use domain::ports::ApprovalRepositoryPort;
use std::sync::Arc;
use uuid::Uuid;

/// Approval Service - 人工审批闸门服务
pub struct ApprovalService {
    repo: Arc<dyn ApprovalRepositoryPort>,
}

impl ApprovalService {
    pub fn new(repo: Arc<dyn ApprovalRepositoryPort>) -> Self {
        Self { repo }
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
        self.repo
            .create(project_id, target_type, target_id, proposed_by, content)
            .await
    }

    /// 批准
    pub async fn approve(
        &self,
        record_id: Uuid,
        reviewer_id: &str,
        comment: Option<&str>,
    ) -> Result<()> {
        self.repo.approve(record_id, reviewer_id, comment).await
    }

    /// 拒绝
    pub async fn reject(
        &self,
        record_id: Uuid,
        reviewer_id: &str,
        comment: Option<&str>,
    ) -> Result<()> {
        self.repo.reject(record_id, reviewer_id, comment).await
    }

    /// 获取待审批记录
    pub async fn get_pending(&self, project_id: Uuid) -> Result<Vec<ApprovalRecord>> {
        self.repo.list_pending(project_id).await
    }
}
