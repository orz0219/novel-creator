//! Proposal Service - 提案管理的业务逻辑层
//!
//! 负责提案的创建、验证、批准、拒绝、提交。
//! 通过 ProposalRepositoryPort 访问数据，不直接依赖 db / sqlx。
//!
//! 批准即提交（提案 九）：approve 时把 ProposedChange 转换为统一的
//! MutationCommand，经 MutationCommitter 落到 World Canon；提交成功后才
//! 把提案状态翻转为 Approved/COMMITTED。提交失败则提案保持待审。

use anyhow::Result;
use domain::mutation::MutationCommand;
use domain::ports::ProposalRepositoryPort;
use domain::validation::{ChangePayload, ProposedChange};
use std::sync::Arc;
use uuid::Uuid;

use crate::mutation::MutationCommitter;

/// Proposal Service - 提案管理服务
pub struct ProposalService {
    repo: Arc<dyn ProposalRepositoryPort>,
    committer: Arc<MutationCommitter>,
}

impl ProposalService {
    pub fn new(repo: Arc<dyn ProposalRepositoryPort>, committer: Arc<MutationCommitter>) -> Self {
        Self { repo, committer }
    }

    /// 列出项目的所有提案
    pub async fn list_proposals(&self, project_id: Uuid) -> Result<Vec<ProposedChange>> {
        self.repo.list_proposals(project_id).await
    }

    /// 获取单个提案
    pub async fn get_proposal(&self, id: Uuid) -> Result<Option<ProposedChange>> {
        self.repo.get_proposal(id).await
    }

    /// 批准并提交流程（提案 九）：转换 -> 提交 Canon -> 标记批准。
    ///
    /// **顺序很重要**：先把数据提交到 Canon，成功后才把提案标记为 Approved。
    /// 反过来（先标 Approved 再提交）会出一个很难发现的坏情况——提交失败时
    /// 提案已经是「已批准」，但数据根本没落库：界面上看起来批过了，
    /// 状态机又不允许再次批准，连重试都做不到。
    /// 实测踩到过：目标实体不存在导致 `fact_entity` 外键失败，就是这个表现。
    ///
    /// 若 Canon 提交失败（冲突 / 校验），直接返回错误，提案保持原状态，可重试。
    pub async fn approve_proposal(&self, id: Uuid) -> Result<ProposedChange> {
        let change = self
            .repo
            .get_proposal(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("proposal {} not found", id))?;

        // 1) 纯计算：payload 解析失败时状态不动
        let cmd = self.to_command(&change)?;

        // 2) 真正写 Canon（提交器只校验命令本身，不依赖提案的状态字段）
        self.committer.commit(cmd).await?;

        // 3) 数据落库成功后才标记批准
        self.repo.approve_proposal(id).await?;

        let updated = self
            .repo
            .get_proposal(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("proposal {} disappeared after commit", id))?;
        Ok(updated)
    }

    /// 把一条 ProposedChange 转换为统一的 MutationCommand。
    fn to_command(&self, change: &ProposedChange) -> Result<MutationCommand> {
        let project_id = change.project_id;
        let target = change.target_entity_id;
        let payload: ChangePayload = serde_json::from_value(change.payload.clone())
            .map_err(|e| anyhow::anyhow!("invalid proposal payload: {}", e))?;

        let cmd = match payload {
            ChangePayload::StateChange { state_key, new_value } => {
                MutationCommand::set_entity_state(project_id, target, &state_key, new_value)
            }
            ChangePayload::EntityCreate {
                entity_type,
                name,
                attributes: _,
            } => {
                // target 视为 world_id（创建实体时尚无 entity id）
                MutationCommand::create_entity(project_id, target, &entity_type, &name, None, None)
            }
            ChangePayload::EntityUpdate {
                name,
                attributes,
            } => MutationCommand::update_entity(
                project_id,
                target,
                None, // 提案驱动：跳过 CAS（expected_version 由实时版本决定）
                name,
                None,
                None,
                attributes,
            ),
            ChangePayload::RelationCreate {
                target_entity_id: tid,
                relation_type,
                attributes: _,
            } => MutationCommand::create_relation(project_id, target, tid, &relation_type, None),
            ChangePayload::KnowledgeUpdate {
                fact_content,
                certainty: _,
            } => MutationCommand::create_fact(project_id, &fact_content, None, Some(vec![target])),
            ChangePayload::Custom(_) => {
                return Err(anyhow::anyhow!(
                    "Custom proposal payload cannot be auto-committed; handle explicitly"
                ));
            }
        };
        Ok(cmd)
    }

    /// 拒绝提案 - 状态转换与 CAS 在 port 实现中验证
    pub async fn reject_proposal(&self, id: Uuid) -> Result<ProposedChange> {
        self.repo.reject_proposal(id).await
    }

    /// 创建提案
    pub async fn create_proposal(
        &self,
        project_id: Uuid,
        task_id: Option<Uuid>,
        change_type: domain::validation::ProposedChangeType,
        target_entity_id: Uuid,
        description: &str,
        payload: serde_json::Value,
    ) -> Result<ProposedChange> {
        self.repo
            .create_proposal(project_id, task_id, change_type, target_entity_id, description, payload)
            .await
    }
}
