//! SceneLedger Repository - 场景账本 CRUD

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use domain::ledger::*;
use sqlx::PgPool;
use uuid::Uuid;

pub struct SceneLedgerRepo {
    pool: PgPool,
}

impl SceneLedgerRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(
        &self,
        project_id: Uuid,
        scene_id: Uuid,
        ledger: &SceneLedger,
    ) -> Result<()> {
        let id = Uuid::new_v4();
        let now = Utc::now();

        sqlx::query(
            "INSERT INTO scene_ledger (id, project_id, scene_id, events, gains, losses, relationship_changes, knowledge_changes, world_changes, foreshadowing_mentions, storyline_progress, character_growth, created_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)",
        )
        .bind(id)
        .bind(project_id)
        .bind(scene_id)
        .bind(serde_json::to_value(&ledger.events).unwrap_or_default())
        .bind(serde_json::to_value(&ledger.gains).unwrap_or_default())
        .bind(serde_json::to_value(&ledger.losses).unwrap_or_default())
        .bind(serde_json::to_value(&ledger.relationship_changes).unwrap_or_default())
        .bind(serde_json::to_value(&ledger.knowledge_changes).unwrap_or_default())
        .bind(serde_json::to_value(&ledger.world_changes).unwrap_or_default())
        .bind(serde_json::to_value(&ledger.foreshadowing_mentions).unwrap_or_default())
        .bind(serde_json::to_value(&ledger.storyline_progress).unwrap_or_default())
        .bind(serde_json::to_value(&ledger.character_growth).unwrap_or_default())
        .bind(now)
        .execute(&self.pool)
        .await
        .context("Failed to insert scene_ledger")?;
        Ok(())
    }

    pub async fn get_by_scene(&self, scene_id: Uuid) -> Result<Option<SceneLedger>> {
        let row = sqlx::query_as::<_, SceneLedgerRow>(
            "SELECT id, project_id, scene_id, events, gains, losses, relationship_changes, knowledge_changes, world_changes, foreshadowing_mentions, storyline_progress, character_growth, created_at \
             FROM scene_ledger WHERE scene_id = $1",
        )
        .bind(scene_id)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to query scene ledger")?;

        Ok(row.map(|r| r.into()))
    }

    pub async fn list_by_project(&self, project_id: Uuid) -> Result<Vec<SceneLedger>> {
        let rows = sqlx::query_as::<_, SceneLedgerRow>(
            "SELECT id, project_id, scene_id, events, gains, losses, relationship_changes, knowledge_changes, world_changes, foreshadowing_mentions, storyline_progress, character_growth, created_at \
             FROM scene_ledger WHERE project_id = $1 ORDER BY created_at",
        )
        .bind(project_id)
        .fetch_all(&self.pool)
        .await
        .context("Failed to query scene ledgers")?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }
}

#[derive(sqlx::FromRow)]
struct SceneLedgerRow {
    id: Uuid,
    project_id: Uuid,
    scene_id: Uuid,
    events: Option<serde_json::Value>,
    gains: Option<serde_json::Value>,
    losses: Option<serde_json::Value>,
    relationship_changes: Option<serde_json::Value>,
    knowledge_changes: Option<serde_json::Value>,
    world_changes: Option<serde_json::Value>,
    foreshadowing_mentions: Option<serde_json::Value>,
    storyline_progress: Option<serde_json::Value>,
    character_growth: Option<serde_json::Value>,
    created_at: DateTime<Utc>,
}

impl From<SceneLedgerRow> for SceneLedger {
    fn from(r: SceneLedgerRow) -> Self {
        let _parse_vec = |v: Option<serde_json::Value>| -> Vec<serde_json::Value> {
            v.and_then(|v| serde_json::from_value(v).ok()).unwrap_or_default()
        };
        SceneLedger {
            id: r.id,
            project_id: r.project_id,
            scene_id: r.scene_id,
            events: r.events.and_then(|v| serde_json::from_value(v).ok()).unwrap_or_default(),
            gains: r.gains.and_then(|v| serde_json::from_value(v).ok()).unwrap_or_default(),
            losses: r.losses.and_then(|v| serde_json::from_value(v).ok()).unwrap_or_default(),
            relationship_changes: r.relationship_changes.and_then(|v| serde_json::from_value(v).ok()).unwrap_or_default(),
            knowledge_changes: r.knowledge_changes.and_then(|v| serde_json::from_value(v).ok()).unwrap_or_default(),
            world_changes: r.world_changes.and_then(|v| serde_json::from_value(v).ok()).unwrap_or_default(),
            foreshadowing_mentions: r.foreshadowing_mentions.and_then(|v| serde_json::from_value(v).ok()).unwrap_or_default(),
            storyline_progress: r.storyline_progress.and_then(|v| serde_json::from_value(v).ok()).unwrap_or_default(),
            character_growth: r.character_growth.and_then(|v| serde_json::from_value(v).ok()).unwrap_or_default(),
            created_at: r.created_at,
        }
    }
}