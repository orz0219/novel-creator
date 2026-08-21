//! Trace Service - AI 可追溯只读视图（generation_run / validation_run）。

use anyhow::Result;
use domain::ports::TraceQueryPort;
use std::sync::Arc;
use uuid::Uuid;

pub struct TraceService {
    repo: Arc<dyn TraceQueryPort>,
}

impl TraceService {
    pub fn new(repo: Arc<dyn TraceQueryPort>) -> Self {
        Self { repo }
    }

    pub async fn list_generation_runs(&self, project_id: Uuid, limit: i64) -> Result<Vec<serde_json::Value>> {
        self.repo.list_generation_runs(project_id, limit).await
    }

    pub async fn list_validation_runs(&self, project_id: Uuid, limit: i64) -> Result<Vec<serde_json::Value>> {
        self.repo.list_validation_runs(project_id, limit).await
    }
}
