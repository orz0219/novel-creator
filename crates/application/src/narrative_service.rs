//! Narrative Service - 叙事管理的业务逻辑层
//!
//! 负责叙事节点的创建、更新、删除（软删除）。
//! 通过 NarrativeRepositoryPort 访问数据，不直接依赖 db / sqlx。

use anyhow::Result;
use domain::mutation::MutationCommand;
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

    pub async fn get_node(&self, id: Uuid) -> Result<Option<Value>> {
        self.repo.get_node(id).await
    }

    pub async fn create_node(
        &self,
        project_id: Uuid,
        node_type: &str,
        parent_id: Option<Uuid>,
        title: &str,
        description: Option<&str>,
        attributes: Value,
    ) -> Result<Value> {
        self.repo
            .create_node(project_id, node_type, parent_id, title, description, attributes)
            .await
    }

    pub async fn update_node(
        &self,
        id: Uuid,
        title: Option<&str>,
        description: Option<&str>,
        content: Option<&str>,
        status: Option<&str>,
    ) -> Result<Value> {
        let project_id = self
            .resolver
            .project_id_for_narrative_node(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("project not found for narrative node {}", id))?;
        let cmd = MutationCommand::update_narrative_node(
            project_id,
            id,
            title.map(|s| s.to_string()),
            description.map(|s| s.to_string()),
            None,
            content.map(|s| s.to_string()),
            status.map(|s| s.to_string()),
        );
        self.committer.commit(cmd).await?;
        self.repo
            .get_node(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("narrative node {} disappeared after update", id))
    }

    pub async fn delete_node(&self, id: Uuid) -> Result<()> {
        let project_id = self
            .resolver
            .project_id_for_narrative_node(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("project not found for narrative node {}", id))?;
        let cmd = MutationCommand::delete_narrative_node(project_id, id);
        self.committer.commit(cmd).await?;
        Ok(())
    }
}
