//! Proposal Service - 提案管理的业务逻辑层
//!
//! 负责提案的创建、验证、批准、拒绝、提交。
//! 通过 ProposalRepositoryPort 访问数据，不直接依赖 db / sqlx。
//! 所有状态转换（含 CAS）已由 port 实现保证，这里只做编排。

use anyhow::Result;
use domain::ports::ProposalRepositoryPort;
use domain::validation::{ProposedChange, ProposedChangeType};
use std::sync::Arc;
use uuid::Uuid;

/// Proposal Service - 提案管理服务
pub struct ProposalService {
    repo: Arc<dyn ProposalRepositoryPort>,
}

impl ProposalService {
    pub fn new(repo: Arc<dyn ProposalRepositoryPort>) -> Self {
        Self { repo }
    }

    /// 列出项目的所有提案
    pub async fn list_proposals(&self, project_id: Uuid) -> Result<Vec<ProposedChange>> {
        self.repo.list_proposals(project_id).await
    }

    /// 获取单个提案
    pub async fn get_proposal(&self, id: Uuid) -> Result<Option<ProposedChange>> {
        self.repo.get_proposal(id).await
    }

    /// 批准提案 - 状态转换与 CAS 在 port 实现中验证
    pub async fn approve_proposal(&self, id: Uuid) -> Result<ProposedChange> {
        self.repo.approve_proposal(id).await
    }

    /// 拒绝提案 - 状态转换与 CAS 在 port 实现中验证
    pub async fn reject_proposal(&self, id: Uuid) -> Result<ProposedChange> {
        self.repo.reject_proposal(id).await
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
        self.repo
            .create_proposal(project_id, task_id, change_type, target_entity_id, description, payload)
            .await
    }
}
