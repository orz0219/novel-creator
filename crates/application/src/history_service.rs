//! History Service - event / fact 的业务逻辑层。
//!
//! 通过 HistoryRepositoryPort 访问数据，不直接依赖 db / sqlx。
//! version 相关为占位，仍由 host 层返回 stub。

use anyhow::Result;
use domain::ports::HistoryRepositoryPort;
use serde_json::Value;
use std::sync::Arc;
use uuid::Uuid;

/// History Service - 历史服务
pub struct HistoryService {
    repo: Arc<dyn HistoryRepositoryPort>,
}

impl HistoryService {
    pub fn new(repo: Arc<dyn HistoryRepositoryPort>) -> Self {
        Self { repo }
    }

    pub async fn list_events(&self, project_id: Uuid, limit: i64) -> Result<Vec<Value>> {
        self.repo.list_events(project_id, limit).await
    }

    pub async fn create_event(
        &self,
        project_id: Uuid,
        name: &str,
        description: &str,
    ) -> Result<Value> {
        self.repo.create_event(project_id, name, description).await
    }

    pub async fn list_facts(&self, project_id: Uuid) -> Result<Vec<Value>> {
        self.repo.list_facts(project_id).await
    }

    pub async fn create_fact(
        &self,
        project_id: Uuid,
        content: &str,
        category: Option<&str>,
        certainty: &str,
    ) -> Result<Value> {
        self.repo
            .create_fact(project_id, content, category, certainty)
            .await
    }
}
