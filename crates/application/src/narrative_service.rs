//! Narrative Service - 叙事管理的业务逻辑层
//!
//! 负责叙事节点（细纲）的创建、更新、删除（软删除）。
//! 通过 NarrativeRepositoryPort 访问数据，不直接依赖 db / sqlx。
//!
//! 两条写路径的分工：
//! - **新建**走 `repo.create_node_full`：细纲字段多（挂线挂阶段 / 挂实体 / 元数据），
//!   而且显式序号要「先让位、后落位」，需要在一个事务内完成；
//! - **修改**走 `MutationCommitter`：文本、状态、结构与挂载一起提交，
//!   才能共用乐观锁（version CAS）与 DomainEvent 记录。

use anyhow::Result;
use domain::mutation::MutationCommand;
use domain::narrative::{NarrativeNodeFilter, NarrativeNodeOutlinePatch, NewNarrativeNode};
use domain::ports::{NarrativeRepositoryPort, ProjectResolverPort};
use serde_json::Value;
use std::sync::Arc;
use uuid::Uuid;

use crate::mutation::MutationCommitter;

/// Narrative Service - 叙事管理服务
pub struct NarrativeService {
    repo: Arc<dyn NarrativeRepositoryPort>,
    committer: Arc<MutationCommitter>,
    resolver: Arc<dyn ProjectResolverPort>,
}

impl NarrativeService {
    pub fn new(
        repo: Arc<dyn NarrativeRepositoryPort>,
        committer: Arc<MutationCommitter>,
        resolver: Arc<dyn ProjectResolverPort>,
    ) -> Self {
        Self {
            repo,
            committer,
            resolver,
        }
    }

    pub async fn list_nodes(&self, project_id: Uuid) -> Result<Vec<Value>> {
        self.repo.list_nodes(project_id).await
    }

    /// 有界 + 有过滤的列表（细纲树下钻）：返回本页 + 命中总数。
    ///
    /// LIMIT/OFFSET 与过滤全部下推到 SQL——细纲到 300 章时「拉全量自己拼树」拉不动。
    pub async fn list_nodes_page(
        &self,
        project_id: Uuid,
        filter: &NarrativeNodeFilter,
        limit: usize,
        offset: usize,
    ) -> Result<(Vec<Value>, usize)> {
        self.repo
            .list_nodes_page(project_id, filter, limit, offset)
            .await
    }

    pub async fn get_node(&self, id: Uuid) -> Result<Option<Value>> {
        self.repo.get_node(id).await
    }

    /// 新建节点（细纲字段齐全）。校验在仓储实现内部完成，缺前置条件直接报错。
    pub async fn create_node(&self, input: NewNarrativeNode) -> Result<Value> {
        self.repo.create_node_full(input).await
    }

    /// 修改节点：文本 / 状态 + 结构与挂载补丁（换父、重排、挂线挂阶段、挂实体、元数据）。
    #[allow(clippy::too_many_arguments)]
    /// 更新叙事节点。
    ///
    /// `attributes` 是**整体替换**语义（`None` = 不改）。它原先被写死成 `None`，
    /// 导致「建节点时漏填的属性再也补不上」——细纲场景要求 objective / conflict /
    /// pov_character_id / location_id 非空，漏一个就只能删了重建，
    /// 而场景一旦被伏笔锚点引用，重建就会让锚点脱钩。
    pub async fn update_node(
        &self,
        id: Uuid,
        title: Option<&str>,
        description: Option<&str>,
        content: Option<&str>,
        status: Option<&str>,
        attributes: Option<serde_json::Value>,
        outline: NarrativeNodeOutlinePatch,
    ) -> Result<Value> {
        let project_id = self
            .resolver
            .project_id_for_narrative_node(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("叙事节点 {} 不属于任何项目", id))?;
        let cmd = MutationCommand::update_narrative_node(
            project_id,
            id,
            title.map(|s| s.to_string()),
            description.map(|s| s.to_string()),
            attributes,
            content.map(|s| s.to_string()),
            status.map(|s| s.to_string()),
            outline,
        );
        self.committer.commit(cmd).await?;
        self.repo
            .get_node(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("叙事节点 {} 更新后读不回来", id))
    }

    pub async fn delete_node(&self, id: Uuid) -> Result<()> {
        let project_id = self
            .resolver
            .project_id_for_narrative_node(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("叙事节点 {} 不属于任何项目", id))?;
        let cmd = MutationCommand::delete_narrative_node(project_id, id);
        self.committer.commit(cmd).await?;
        Ok(())
    }
}
