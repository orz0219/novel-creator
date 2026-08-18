//! Contract Repository - CRUD operations for SceneContract

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use domain::SceneContract;
use sqlx::PgPool;
use uuid::Uuid;

pub struct ContractRepo {
    pool: PgPool,
}

impl ContractRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 创建场景契约
    pub async fn create(
        &self,
        scene_id: Uuid,
        required_events: Vec<String>,
        forbidden_events: Vec<String>,
        required_characters: Vec<Uuid>,
        required_facts: Vec<String>,
        reader_learns: Vec<String>,
        protagonist_learns: Vec<String>,
        world_changes: Vec<String>,
    ) -> Result<SceneContract> {
        let id = Uuid::new_v4();
        let now = Utc::now();

        sqlx::query(
            "INSERT INTO scene_contract (id, scene_id, required_events, forbidden_events, required_characters, required_facts, reader_learns, protagonist_learns, world_changes, created_at, updated_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
        )
        .bind(id)
        .bind(scene_id)
        .bind(serde_json::to_value(&required_events).unwrap_or_default())
        .bind(serde_json::to_value(&forbidden_events).unwrap_or_default())
        .bind(serde_json::to_value(&required_characters).unwrap_or_default())
        .bind(serde_json::to_value(&required_facts).unwrap_or_default())
        .bind(serde_json::to_value(&reader_learns).unwrap_or_default())
        .bind(serde_json::to_value(&protagonist_learns).unwrap_or_default())
        .bind(serde_json::to_value(&world_changes).unwrap_or_default())
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await
        .context("Failed to create scene contract")?;

        Ok(SceneContract {
            id,
            scene_id,
            required_events,
            forbidden_events,
            required_characters,
            required_facts,
            reader_learns,
            protagonist_learns,
            world_changes,
            created_at: now,
            updated_at: now,
        })
    }

    /// 按场景获取契约
    pub async fn get_by_scene(&self, scene_id: Uuid) -> Result<Option<SceneContract>> {
        let row = sqlx::query_as::<_, SceneContractRow>(
            "SELECT id, scene_id, required_events, forbidden_events, required_characters, required_facts, reader_learns, protagonist_learns, world_changes, created_at, updated_at \
             FROM scene_contract WHERE scene_id = $1",
        )
        .bind(scene_id)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to query scene contract")?;

        Ok(row.map(|r| r.into()))
    }
}

#[derive(sqlx::FromRow)]
struct SceneContractRow {
    id: Uuid,
    scene_id: Uuid,
    required_events: Option<serde_json::Value>,
    forbidden_events: Option<serde_json::Value>,
    required_characters: Option<serde_json::Value>,
    required_facts: Option<serde_json::Value>,
    reader_learns: Option<serde_json::Value>,
    protagonist_learns: Option<serde_json::Value>,
    world_changes: Option<serde_json::Value>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<SceneContractRow> for SceneContract {
    fn from(r: SceneContractRow) -> Self {
        let parse_vec = |v: Option<serde_json::Value>| -> Vec<String> {
            v.and_then(|v| serde_json::from_value(v).ok()).unwrap_or_default()
        };
        let parse_uuid_vec = |v: Option<serde_json::Value>| -> Vec<Uuid> {
            v.and_then(|v| serde_json::from_value(v).ok()).unwrap_or_default()
        };
        SceneContract {
            id: r.id,
            scene_id: r.scene_id,
            required_events: parse_vec(r.required_events),
            forbidden_events: parse_vec(r.forbidden_events),
            required_characters: parse_uuid_vec(r.required_characters),
            required_facts: parse_vec(r.required_facts),
            reader_learns: parse_vec(r.reader_learns),
            protagonist_learns: parse_vec(r.protagonist_learns),
            world_changes: parse_vec(r.world_changes),
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}
