//! NarrativeBudget Repository - Word count allocation tracking

use anyhow::{Context, Result};
use chrono::Utc;
use uuid::Uuid;

use crate::connection::Database;
use crate::time_utils;

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
    pub created_at: chrono::DateTime<Utc>,
    pub updated_at: chrono::DateTime<Utc>,
}

pub struct NarrativeBudgetRepo<'a> {
    db: &'a Database,
}

impl<'a> NarrativeBudgetRepo<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    pub fn create(
        &self,
        project_id: Uuid,
        narrative_node_id: Uuid,
        allocated_words: i32,
    ) -> Result<NarrativeBudgetRecord> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let conn = self.db.conn();
        conn.execute(
            "INSERT INTO narrative_budget (id, project_id, narrative_node_id, allocated_words, used_words, pacing_warning_threshold, created_at, updated_at) VALUES (?, ?, ?, ?, 0, 0.9, ?, ?)",
            [id.to_string(), project_id.to_string(), narrative_node_id.to_string(), allocated_words.to_string(), now.to_string(), now.to_string()],
        ).context("Failed to create narrative_budget")?;
        Ok(NarrativeBudgetRecord {
            id, project_id, narrative_node_id, allocated_words,
            used_words: 0, action_ratio: None, dialogue_ratio: None,
            description_ratio: None, exposition_ratio: None,
            internal_monologue_ratio: None, pacing_warning_threshold: 0.9,
            created_at: now, updated_at: now,
        })
    }

    pub fn get_by_node(&self, narrative_node_id: Uuid) -> Result<Option<NarrativeBudgetRecord>> {
        let conn = self.db.conn();
        let result = conn.query_row(
            "SELECT id, project_id, narrative_node_id, allocated_words, used_words, action_ratio, dialogue_ratio, description_ratio, exposition_ratio, internal_monologue_ratio, pacing_warning_threshold, created_at, updated_at FROM narrative_budget WHERE narrative_node_id = ?",
            [narrative_node_id.to_string()],
            |row| {
                Ok(NarrativeBudgetRecord {
                    id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
                    project_id: Uuid::parse_str(&row.get::<_, String>(1)?).unwrap(),
                    narrative_node_id: Uuid::parse_str(&row.get::<_, String>(2)?).unwrap(),
                    allocated_words: row.get(3)?,
                    used_words: row.get(4)?,
                    action_ratio: row.get(5)?,
                    dialogue_ratio: row.get(6)?,
                    description_ratio: row.get(7)?,
                    exposition_ratio: row.get(8)?,
                    internal_monologue_ratio: row.get(9)?,
                    pacing_warning_threshold: row.get(10)?,
                    created_at: time_utils::get_timestamp(row, 11),
                    updated_at: time_utils::get_timestamp(row, 12),
                })
            },
        ).ok();
        Ok(result)
    }

    pub fn add_words(&self, narrative_node_id: Uuid, words: i32) -> Result<()> {
        let conn = self.db.conn();
        let now = Utc::now();
        conn.execute(
            "UPDATE narrative_budget SET used_words = used_words + ?, updated_at = ? WHERE narrative_node_id = ?",
            [words.to_string(), now.to_string(), narrative_node_id.to_string()],
        ).context("Failed to update narrative_budget")?;
        Ok(())
    }

    pub fn check_pacing_warning(&self, narrative_node_id: Uuid) -> Result<Option<String>> {
        if let Some(budget) = self.get_by_node(narrative_node_id)? {
            if budget.allocated_words > 0 {
                let usage_ratio = budget.used_words as f64 / budget.allocated_words as f64;
                if usage_ratio >= budget.pacing_warning_threshold {
                    return Ok(Some(format!(
                        "PACING_WARNING: Node {} has used {:.0}% of allocated words ({}/{})",
                        narrative_node_id, usage_ratio * 100.0, budget.used_words, budget.allocated_words
                    )));
                }
            }
        }
        Ok(None)
    }
}
