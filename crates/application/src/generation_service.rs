//! Generation Service - 生成任务管理的业务逻辑层
//!
//! 负责生成任务的创建、查询、取消。
//! 通过 GenerationRepositoryPort 访问数据，不直接依赖 db / sqlx。

use anyhow::Result;
use domain::ports::GenerationRepositoryPort;
use serde_json::Value;
use std::sync::Arc;
use uuid::Uuid;

/// Generation Service - 生成任务管理服务
pub struct GenerationService {
    repo: Arc<dyn GenerationRepositoryPort>,
}

impl GenerationService {
    pub fn new(repo: Arc<dyn GenerationRepositoryPort>) -> Self {
        Self { repo }
    }

    /// 列出项目的生成任务
    pub async fn list_tasks(&self, project_id: Uuid) -> Result<Vec<Value>> {
        self.repo.list_tasks(project_id).await
    }

    /// 获取单个生成任务
    pub async fn get_task(&self, id: Uuid) -> Result<Option<Value>> {
        self.repo.get_task(id).await
    }

    /// 创建生成任务
    pub async fn create_task(
        &self,
        project_id: Uuid,
        task_type: &str,
        target_id: Option<Uuid>,
        model: Option<&str>,
        parameters: Value,
    ) -> Result<Value> {
        self.repo
            .create_task(project_id, task_type, target_id, model, parameters)
            .await
    }

    /// 取消生成任务（只允许取消 Pending 或 Running 状态的任务）
    pub async fn cancel_task(&self, id: Uuid) -> Result<()> {
        self.repo.cancel_task(id).await
    }
}
