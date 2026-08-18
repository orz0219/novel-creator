//! Narrative Budget - Word count allocation per narrative node
//!
//! Design Doc 6: Each Volume/Arc/Chapter/Scene has word count budget.
//! System can detect "this Arc used 90% words but only completed 30% objectives".

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Narrative Budget - tracks word count allocation and usage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NarrativeBudget {
    pub id: Uuid,
    pub project_id: Uuid,
    pub narrative_node_id: Uuid,
    /// Allocated word count
    pub allocated_words: i32,
    /// Used word count
    pub used_words: i32,
    /// Action description ratio
    pub action_ratio: Option<f64>,
    /// Dialogue ratio
    pub dialogue_ratio: Option<f64>,
    /// Description ratio
    pub description_ratio: Option<f64>,
    /// Exposition ratio
    pub exposition_ratio: Option<f64>,
    /// Internal monologue ratio
    pub internal_monologue_ratio: Option<f64>,
    /// Warning threshold (0.0-1.0)
    pub pacing_warning_threshold: f64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl NarrativeBudget {
    pub fn new(project_id: Uuid, narrative_node_id: Uuid, allocated_words: i32) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
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
        }
    }

    /// Usage ratio (0.0 - 1.0+)
    pub fn usage_ratio(&self) -> f64 {
        if self.allocated_words == 0 {
            return 0.0;
        }
        self.used_words as f64 / self.allocated_words as f64
    }

    /// Check if pacing warning should trigger
    pub fn should_warn(&self) -> bool {
        self.usage_ratio() >= self.pacing_warning_threshold
    }

    /// Add words to the budget
    pub fn add_words(&mut self, words: i32) {
        self.used_words += words;
        self.updated_at = Utc::now();
    }
}

/// Pacing Warning types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PacingWarning {
    /// Over budget
    OverBudget { node_id: Uuid, used: i32, allocated: i32 },
    /// Pacing imbalance (too much action, not enough dialogue, etc)
    PacingImbalance { node_id: Uuid, dimension: String, ratio: f64 },
    /// Objectives not met despite high word usage
    ObjectiveMismatch { node_id: Uuid, word_usage: f64, objective_completion: f64 },
}
