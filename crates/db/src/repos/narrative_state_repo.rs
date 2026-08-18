//! NarrativeState Repository - 叙事状态 CRUD

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use domain::character_mind::{NarrativeState, StateDimension};
use sqlx::PgPool;
use uuid::Uuid;

pub struct NarrativeStateRepo {
    pool: PgPool,
}

impl NarrativeStateRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(
        &self,
        project_id: Uuid,
        dimension: StateDimension,
        state_key: &str,
        state_value: serde_json::Value,
        scene_id: Option<Uuid>,
    ) -> Result<NarrativeState> {
        let id = Uuid::new_v4();
        let now = Utc::now();

        sqlx::query(
            "INSERT INTO narrative_state (id, project_id, state_dimension, state_key, state_value, scene_id, created_at, updated_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(id)
        .bind(project_id)
        .bind(dimension.as_str())
        .bind(state_key)
        .bind(&state_value)
        .bind(scene_id)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await
        .context("Failed to insert narrative_state")?;

        Ok(NarrativeState {
            id,
            project_id,
            state_dimension: dimension,
            state_key: state_key.to_string(),
            state_value,
            scene_id,
            created_at: now,
            updated_at: now,
        })
    }

    pub async fn list_by_dimension(
        &self,
        project_id: Uuid,
        dimension: StateDimension,
    ) -> Result<Vec<NarrativeState>> {
        let rows = sqlx::query_as::<_, NarrativeStateRow>(
            "SELECT id, project_id, state_dimension, state_key, state_value, scene_id, created_at, updated_at \
             FROM narrative_state WHERE project_id = $1 AND state_dimension = $2",
        )
        .bind(project_id)
        .bind(dimension.as_str())
        .fetch_all(&self.pool)
        .await
        .context("Failed to query narrative states")?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    pub async fn get_by_key(
        &self,
        project_id: Uuid,
        dimension: StateDimension,
        state_key: &str,
    ) -> Result<Option<NarrativeState>> {
        let row = sqlx::query_as::<_, NarrativeStateRow>(
            "SELECT id, project_id, state_dimension, state_key, state_value, scene_id, created_at, updated_at \
             FROM narrative_state WHERE project_id = $1 AND state_dimension = $2 AND state_key = $3 \
             ORDER BY updated_at DESC LIMIT 1",
        )
        .bind(project_id)
        .bind(dimension.as_str())
        .bind(state_key)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to query narrative state")?;

        Ok(row.map(|r| r.into()))
    }
}

#[derive(sqlx::FromRow)]
struct NarrativeStateRow {
    id: Uuid,
    project_id: Uuid,
    state_dimension: String,
    state_key: String,
    state_value: Option<serde_json::Value>,
    scene_id: Option<Uuid>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<NarrativeStateRow> for NarrativeState {
    fn from(r: NarrativeStateRow) -> Self {
        NarrativeState {
            id: r.id,
            project_id: r.project_id,
            state_dimension: StateDimension::from_str(&r.state_dimension),
            state_key: r.state_key,
            state_value: r.state_value.unwrap_or(serde_json::json!(null)),
            scene_id: r.scene_id,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}
