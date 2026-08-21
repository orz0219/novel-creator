//! Foreshadow Service - 伏笔管理的业务逻辑层。
//!
//! 通过 ForeshadowRepositoryPort 访问数据，不直接依赖 db / sqlx。

use anyhow::Result;
use domain::ports::ForeshadowRepositoryPort;
use serde_json::Value;
use std::sync::Arc;
use uuid::Uuid;

/// Foreshadow Service - 伏笔服务
pub struct ForeshadowService {
    repo: Arc<dyn ForeshadowRepositoryPort>,
}

impl ForeshadowService {
    pub fn new(repo: Arc<dyn ForeshadowRepositoryPort>) -> Self {
        Self { repo }
    }

    pub async fn list_foreshadows(&self, project_id: Uuid) -> Result<Vec<Value>> {
        self.repo.list_foreshadows(project_id).await
    }

    pub async fn create_foreshadow(
        &self,
        project_id: Uuid,
        name: &str,
        description: Option<&str>,
        importance: &str,
        hint_level: &str,
    ) -> Result<Value> {
        self.repo
            .create_foreshadow(project_id, name, description, importance, hint_level)
            .await
    }

    pub async fn update_foreshadow(
        &self,
        id: Uuid,
        name: &str,
        description: Option<&str>,
    ) -> Result<Value> {
        self.repo.update_foreshadow(id, name, description).await
    }

    /// 删除伏笔（按 id）。
    pub async fn delete_foreshadow(&self, id: Uuid) -> Result<()> {
        self.repo.delete_foreshadow(id).await
    }
}
