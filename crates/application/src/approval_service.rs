//! Approval Service - Human Approval Gate 业务逻辑

use anyhow::Result;
use db::connection::Database;
use db::repos::approval_repo::ApprovalRepo;
use domain::*;
use uuid::Uuid;

pub struct ApprovalService<'a> {
    db: &'a Database,
}

impl<'a> ApprovalService<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    /// 提交审批请求
    pub fn submit_for_approval(&self, project_id: Uuid, target_type: ApprovalTargetType, target_id: Uuid, proposed_by: &str, content: serde_json::Value) -> Result<ApprovalRecord> {
        let repo = ApprovalRepo::new(self.db);
        repo.create(project_id, target_type, target_id, proposed_by, content)
    }

    /// 批准
    pub fn approve(&self, record_id: Uuid, reviewer_id: &str, comment: Option<&str>) -> Result<()> {
        let repo = ApprovalRepo::new(self.db);
        repo.approve(record_id, reviewer_id, comment)
    }

    /// 拒绝
    pub fn reject(&self, record_id: Uuid, reviewer_id: &str, comment: Option<&str>) -> Result<()> {
        let repo = ApprovalRepo::new(self.db);
        repo.reject(record_id, reviewer_id, comment)
    }

    /// 获取待审批记录
    pub fn get_pending(&self, project_id: Uuid) -> Result<Vec<ApprovalRecord>> {
        let repo = ApprovalRepo::new(self.db);
        repo.list_pending(project_id)
    }
}
