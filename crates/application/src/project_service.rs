//! Project Service - 项目管理的业务逻辑层。
//!
//! 通过 ProjectRepositoryPort 访问数据，不直接依赖 db / sqlx。
//! 具体 SQL 与事务实现已下沉到 db crate 的 port 实现。

use anyhow::Result;
use domain::ports::ProjectRepositoryPort;
use serde_json::Value;
use std::sync::Arc;
use uuid::Uuid;

use crate::world_service::WorldService;

/// Project Service - 项目服务
pub struct ProjectService {
    repo: Arc<dyn ProjectRepositoryPort>,
    /// 主世界服务：建项目时自动 ensure 一个主世界，供实体 / 规则按项目物理隔离。
    world: Arc<WorldService>,
}

impl ProjectService {
    pub fn new(repo: Arc<dyn ProjectRepositoryPort>, world: Arc<WorldService>) -> Self {
        Self { repo, world }
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
        let v = self.repo.create_project(name, description, language).await?;
        let id = v
            .get("id")
            .and_then(|x| x.as_str())
            .and_then(|s| Uuid::parse_str(s).ok())
            .ok_or_else(|| anyhow::anyhow!("创建项目后未返回有效 id"))?;
        // 自动 ensure 主世界，使该项目的实体 / 规则工具可按要求按 project_id 物理隔离。
        self.world.ensure_main_world(id, name).await?;
        Ok(v)
    }

    pub async fn update_project(
        &self,
        id: Uuid,
        name: Option<&str>,
        description: Option<&str>,
        status: Option<&str>,
        premise: Option<&str>,
    ) -> Result<Value> {
        self.repo.update_project(id, name, description, status, premise).await
    }

    pub async fn delete_project(&self, id: Uuid) -> Result<()> {
        self.repo.delete_project(id).await
    }
}
