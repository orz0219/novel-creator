//! DecisionTrace Repository - AI 决策追踪 CRUD

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use domain::ledger::*;
use sqlx::PgPool;
use uuid::Uuid;

pub struct DecisionTraceRepo {
    pool: PgPool,
}

impl DecisionTraceRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(
        &self,
        project_id: Uuid,
        scene_id: Uuid,
        character_id: Uuid,
        decision: &str,
        factors: Vec<DecisionFactor>,
    ) -> Result<DecisionTrace> {
        let id = Uuid::new_v4();
        let now = Utc::now();

        sqlx::query(
            "INSERT INTO decision_trace (id, project_id, scene_id, character_id, decision, factors, created_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(id)
        .bind(project_id)
        .bind(scene_id)
        .bind(character_id)
        .bind(decision)
        .bind(serde_json::to_value(&factors).unwrap_or_default())
        .bind(now)
        .execute(&self.pool)
        .await
        .context("Failed to insert decision_trace")?;

        Ok(DecisionTrace {
            id,
            project_id,
            scene_id,
            character_id,
            decision: decision.to_string(),
            factors,
            created_at: now,
        })
    }

    pub async fn list_by_scene(&self, scene_id: Uuid) -> Result<Vec<DecisionTrace>> {
        let rows = sqlx::query_as::<_, DecisionTraceRow>(
            "SELECT id, project_id, scene_id, character_id, decision, factors, created_at \
             FROM decision_trace WHERE scene_id = $1",
        )
        .bind(scene_id)
        .fetch_all(&self.pool)
        .await
        .context("Failed to query decision traces")?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    pub async fn list_by_character(&self, character_id: Uuid) -> Result<Vec<DecisionTrace>> {
        let rows = sqlx::query_as::<_, DecisionTraceRow>(
            "SELECT id, project_id, scene_id, character_id, decision, factors, created_at \
             FROM decision_trace WHERE character_id = $1 ORDER BY created_at DESC",
        )
        .bind(character_id)
        .fetch_all(&self.pool)
        .await
        .context("Failed to query decision traces")?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }
}

#[derive(sqlx::FromRow)]
struct DecisionTraceRow {
    id: Uuid,
    project_id: Uuid,
    scene_id: Uuid,
    character_id: Uuid,
    decision: String,
    factors: Option<serde_json::Value>,
    created_at: DateTime<Utc>,
}

impl From<DecisionTraceRow> for DecisionTrace {
    fn from(r: DecisionTraceRow) -> Self {
        DecisionTrace {
            id: r.id,
            project_id: r.project_id,
            scene_id: r.scene_id,
            character_id: r.character_id,
            decision: r.decision,
            factors: r.factors.and_then(|v| serde_json::from_value(v).ok()).unwrap_or_default(),
            created_at: r.created_at,
        }
    }
}
