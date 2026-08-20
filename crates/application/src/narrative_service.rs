//! Narrative Service - 叙事管理的业务逻辑层
//!
//! 负责叙事节点的创建、更新、删除（软删除）。
//! 通过 NarrativeRepositoryPort 访问数据，不直接依赖 db / sqlx。

use anyhow::Result;
use domain::ports::NarrativeRepositoryPort;
use serde_json::Value;
use std::sync::Arc;
use uuid::Uuid;

/// Narrative Service - 叙事管理服务
pub struct NarrativeService {
    repo: Arc<dyn NarrativeRepositoryPort>,
}

impl NarrativeService {
    pub fn new(repo: Arc<dyn NarrativeRepositoryPort>) -> Self {
        Self { repo }
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
        status: Option<&str>,
    ) -> Result<Value> {
        self.repo.update_node(id, title, description, status).await
    }

    pub async fn delete_node(&self, id: Uuid) -> Result<()> {
        self.repo.delete_node(id).await
    }
}
