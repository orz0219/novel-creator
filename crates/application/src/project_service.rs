//! Project Service - 项目管理的业务逻辑层。
//!
//! 通过 ProjectRepositoryPort 访问数据，不直接依赖 db / sqlx。
//! 具体 SQL 与事务实现已下沉到 db crate 的 port 实现。

use anyhow::Result;
use domain::ports::ProjectRepositoryPort;
use serde_json::Value;
use std::sync::Arc;
use uuid::Uuid;

/// Project Service - 项目服务
pub struct ProjectService {
    repo: Arc<dyn ProjectRepositoryPort>,
}

impl ProjectService {
    pub fn new(repo: Arc<dyn ProjectRepositoryPort>) -> Self {
        Self { repo }
    }

    pub async fn list_projects(&self) -> Result<Vec<Value>> {
        self.repo.list_projects().await
    }

    pub async fn get_project(&self, id: Uuid) -> Result<Option<Value>> {
        self.repo.get_project(id).await
    }

    pub async fn create_project(
        &self,
        name: &str,
        description: Option<&str>,
        language: Option<&str>,
    ) -> Result<Value> {
        self.repo.create_project(name, description, language).await
    }

    pub async fn update_project(
        &self,
        id: Uuid,
        name: Option<&str>,
        description: Option<&str>,
        status: Option<&str>,
    ) -> Result<Value> {
        self.repo.update_project(id, name, description, status).await
    }

    pub async fn delete_project(&self, id: Uuid) -> Result<()> {
        self.repo.delete_project(id).await
    }
}
