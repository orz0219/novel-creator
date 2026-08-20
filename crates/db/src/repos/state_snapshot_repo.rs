//! StateSnapshot Repository - 状态快照（回滚支持）

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use ai::state_mgmt::*;
use sqlx::PgPool;
use uuid::Uuid;

pub struct StateSnapshotRepo {
    pool: PgPool,
}

impl StateSnapshotRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(
        &self,
        project_id: Uuid,
        scene_id: Uuid,
        state_before: serde_json::Value,
        changes: serde_json::Value,
        state_after: serde_json::Value,
    ) -> Result<StateSnapshot> {
        let id = Uuid::new_v4();
        let now = Utc::now();

        sqlx::query(
            "INSERT INTO state_snapshot (id, project_id, scene_id, state_before, changes, state_after, created_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(id)
        .bind(project_id)
        .bind(scene_id)
        .bind(&state_before)
        .bind(&changes)
        .bind(&state_after)
        .bind(now)
        .execute(&self.pool)
        .await
        .context("Failed to insert state_snapshot")?;

        Ok(StateSnapshot {
            id,
            project_id,
            scene_id,
            state_before,
            changes,
            state_after,
            created_at: now,
        })
    }

    pub async fn get_by_scene(&self, scene_id: Uuid) -> Result<Option<StateSnapshot>> {
        let row = sqlx::query_as::<_, StateSnapshotRow>(
            "SELECT id, project_id, scene_id, state_before, changes, state_after, created_at \
             FROM state_snapshot WHERE scene_id = $1",
        )
        .bind(scene_id)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to query state snapshot")?;

        Ok(row.map(|r| r.into()))
    }

    pub async fn list_by_project(&self, project_id: Uuid) -> Result<Vec<StateSnapshot>> {
        let rows = sqlx::query_as::<_, StateSnapshotRow>(
            "SELECT id, project_id, scene_id, state_before, changes, state_after, created_at \
             FROM state_snapshot WHERE project_id = $1 ORDER BY created_at",
        )
        .bind(project_id)
        .fetch_all(&self.pool)
        .await
        .context("Failed to query state snapshots")?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    /// 获取指定场景之前的所有快照（用于回滚）
    pub async fn list_before_scene(
        &self,
        project_id: Uuid,
        scene_id: Uuid,
    ) -> Result<Vec<StateSnapshot>> {
        let rows = sqlx::query_as::<_, StateSnapshotRow>(
            "SELECT id, project_id, scene_id, state_before, changes, state_after, created_at \
             FROM state_snapshot WHERE project_id = $1 AND scene_id != $2 ORDER BY created_at DESC",
        )
        .bind(project_id)
        .bind(scene_id)
        .fetch_all(&self.pool)
        .await
        .context("Failed to query state snapshots")?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }
}

#[derive(sqlx::FromRow)]
struct StateSnapshotRow {
    id: Uuid,
    project_id: Uuid,
    scene_id: Uuid,
    state_before: Option<serde_json::Value>,
    changes: Option<serde_json::Value>,
    state_after: Option<serde_json::Value>,
    created_at: DateTime<Utc>,
}

impl From<StateSnapshotRow> for StateSnapshot {
    fn from(r: StateSnapshotRow) -> Self {
        StateSnapshot {
            id: r.id,
            project_id: r.project_id,
            scene_id: r.scene_id,
            state_before: r.state_before.unwrap_or(serde_json::json!({})),
            changes: r.changes.unwrap_or(serde_json::json!({})),
            state_after: r.state_after.unwrap_or(serde_json::json!({})),
            created_at: r.created_at,
        }
    }
}
