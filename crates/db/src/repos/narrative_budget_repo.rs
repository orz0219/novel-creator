//! NarrativeBudget Repository - Word count allocation tracking

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

/// NarrativeBudget - tracks word count allocation and usage per narrative node
#[derive(Debug, Clone)]
pub struct NarrativeBudgetRecord {
    pub id: Uuid,
    pub project_id: Uuid,
    pub narrative_node_id: Uuid,
    pub allocated_words: i32,
    pub used_words: i32,
    pub action_ratio: Option<f64>,
    pub dialogue_ratio: Option<f64>,
    pub description_ratio: Option<f64>,
    pub exposition_ratio: Option<f64>,
    pub internal_monologue_ratio: Option<f64>,
    pub pacing_warning_threshold: f64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct NarrativeBudgetRepo {
    pool: PgPool,
}

impl NarrativeBudgetRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(
        &self,
        project_id: Uuid,
        narrative_node_id: Uuid,
        allocated_words: i32,
    ) -> Result<NarrativeBudgetRecord> {
        let id = Uuid::new_v4();
        let now = Utc::now();

        sqlx::query(
            "INSERT INTO narrative_budget (id, project_id, narrative_node_id, allocated_words, used_words, pacing_warning_threshold, created_at, updated_at) \
             VALUES ($1, $2, $3, $4, 0, 0.9, $5, $6)",
        )
        .bind(id)
        .bind(project_id)
        .bind(narrative_node_id)
        .bind(allocated_words)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await
        .context("Failed to create narrative_budget")?;

        Ok(NarrativeBudgetRecord {
            id,
            project_id,
            narrative_node_id,
            allocated_words,
            used_words: 0,
            action_ratio: None,
            dialogue_ratio: None,
            description_ratio: None,
            exposition_ratio: None,
            internal_monologue_ratio: None,
            pacing_warning_threshold: 0.9,
            created_at: now,
            updated_at: now,
        })
    }

    pub async fn get_by_node(
        &self,
        narrative_node_id: Uuid,
    ) -> Result<Option<NarrativeBudgetRecord>> {
        let row = sqlx::query_as::<_, NarrativeBudgetRow>(
            "SELECT id, project_id, narrative_node_id, allocated_words, used_words, action_ratio, dialogue_ratio, description_ratio, exposition_ratio, internal_monologue_ratio, pacing_warning_threshold, created_at, updated_at \
             FROM narrative_budget WHERE narrative_node_id = $1",
        )
        .bind(narrative_node_id)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to query narrative_budget")?;

        Ok(row.map(|r| r.into()))
    }

    pub async fn add_words(&self, narrative_node_id: Uuid, words: i32) -> Result<()> {
        let now = Utc::now();

        sqlx::query(
            "UPDATE narrative_budget SET used_words = used_words + $1, updated_at = $2 WHERE narrative_node_id = $3",
        )
        .bind(words)
        .bind(now)
        .bind(narrative_node_id)
        .execute(&self.pool)
        .await
        .context("Failed to update narrative_budget")?;
        Ok(())
    }

    pub async fn check_pacing_warning(
        &self,
        narrative_node_id: Uuid,
    ) -> Result<Option<String>> {
        if let Some(budget) = self.get_by_node(narrative_node_id).await? {
            if budget.allocated_words > 0 {
                let usage_ratio = budget.used_words as f64 / budget.allocated_words as f64;
                if usage_ratio >= budget.pacing_warning_threshold {
                    return Ok(Some(format!(
                        "PACING_WARNING: Node {} has used {:.0}% of allocated words ({}/{})",
                        narrative_node_id,
                        usage_ratio * 100.0,
                        budget.used_words,
                        budget.allocated_words
                    )));
                }
            }
        }
        Ok(None)
    }
}

#[derive(sqlx::FromRow)]
struct NarrativeBudgetRow {
    id: Uuid,
    project_id: Uuid,
    narrative_node_id: Uuid,
    allocated_words: i32,
    used_words: i32,
    action_ratio: Option<f64>,
    dialogue_ratio: Option<f64>,
    description_ratio: Option<f64>,
    exposition_ratio: Option<f64>,
    internal_monologue_ratio: Option<f64>,
    pacing_warning_threshold: f64,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<NarrativeBudgetRow> for NarrativeBudgetRecord {
    fn from(r: NarrativeBudgetRow) -> Self {
        NarrativeBudgetRecord {
            id: r.id,
            project_id: r.project_id,
            narrative_node_id: r.narrative_node_id,
            allocated_words: r.allocated_words,
            used_words: r.used_words,
            action_ratio: r.action_ratio,
            dialogue_ratio: r.dialogue_ratio,
            description_ratio: r.description_ratio,
            exposition_ratio: r.exposition_ratio,
            internal_monologue_ratio: r.internal_monologue_ratio,
            pacing_warning_threshold: r.pacing_warning_threshold,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}
