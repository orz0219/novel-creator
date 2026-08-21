//! Snapshot Service - novel_state_snapshot 的业务逻辑层。
//!
//! 通过 SnapshotRepositoryPort / NarrativeStateWritePort 访问数据，不直接依赖 db / sqlx。

use anyhow::{bail, Result};
use domain::narrative::StateDimension;
use domain::ports::{NarrativeStateWritePort, SnapshotRepositoryPort};
use serde_json::Value;
use std::sync::Arc;
use uuid::Uuid;

/// Snapshot Service - 快照服务
pub struct SnapshotService {
    repo: Arc<dyn SnapshotRepositoryPort>,
    state_writer: Arc<dyn NarrativeStateWritePort>,
}

impl SnapshotService {
    pub fn new(
        repo: Arc<dyn SnapshotRepositoryPort>,
        state_writer: Arc<dyn NarrativeStateWritePort>,
    ) -> Self {
        Self { repo, state_writer }
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

    /// 恢复快照：把快照的宏观状态回写到 narrative_state（World 维度）。
    ///
    /// 写入键：story_time / world_summary / main_character_state / current_location，
    /// 以及完整 state_data（原样 JSON）。已存在的同键状态会被更新（幂等），不删除任何现有数据。
    ///
    /// 返回恢复摘要；快照不存在返回错误（API 层转 404/400）。
    pub async fn restore_snapshot(&self, id: Uuid) -> Result<Value> {
        let snapshot = match self.repo.find_snapshot(id).await? {
            Some(s) => s,
            None => bail!("Snapshot not found"),
        };

        let project_id: Uuid = snapshot["project_id"]
            .as_str()
            .and_then(|s| Uuid::parse_str(s).ok())
            .ok_or_else(|| anyhow::anyhow!("Snapshot has invalid project_id"))?;

        let mut restored_keys: Vec<&str> = Vec::new();

        for (key, dimension) in [
            ("story_time", StateDimension::World),
            ("world_summary", StateDimension::World),
            ("main_character_state", StateDimension::World),
            ("current_location", StateDimension::World),
        ] {
            if let Some(v) = snapshot[key].as_str() {
                if !v.is_empty() {
                    let value = Value::String(v.to_string());
                    self.state_writer
                        .upsert_state(project_id, dimension, key, value)
                        .await?;
                    restored_keys.push(key);
                }
            }
        }

        if let Some(state_data) = snapshot.get("state_data") {
            if !state_data.is_null() {
                self.state_writer
                    .upsert_state(
                        project_id,
                        StateDimension::World,
                        "snapshot_state_data",
                        state_data.clone(),
                    )
                    .await?;
                restored_keys.push("snapshot_state_data");
            }
        }

        Ok(serde_json::json!({
            "restored": true,
            "snapshot_id": id.to_string(),
            "project_id": project_id.to_string(),
            "restored_keys": restored_keys,
        }))
    }
}
