//! Snapshot Service - novel_state_snapshot 的业务逻辑层。
//!
//! 通过 SnapshotRepositoryPort 访问数据，不直接依赖 db / sqlx。

use anyhow::Result;
use domain::ports::SnapshotRepositoryPort;
use serde_json::Value;
use std::sync::Arc;
use uuid::Uuid;

/// Snapshot Service - 快照服务
pub struct SnapshotService {
    repo: Arc<dyn SnapshotRepositoryPort>,
}

impl SnapshotService {
    pub fn new(repo: Arc<dyn SnapshotRepositoryPort>) -> Self {
        Self { repo }
    }

    pub async fn list_snapshots(&self, project_id: Uuid) -> Result<Vec<Value>> {
        self.repo.list_snapshots(project_id).await
    }

    pub async fn create_snapshot(
        &self,
        project_id: Uuid,
        name: Option<&str>,
        story_time: Option<&str>,
        world_summary: Option<&str>,
    ) -> Result<Value> {
        self.repo
            .create_snapshot(project_id, name, story_time, world_summary)
            .await
    }

    pub async fn delete_snapshot(&self, id: Uuid) -> Result<()> {
        self.repo.delete_snapshot(id).await
    }
}
