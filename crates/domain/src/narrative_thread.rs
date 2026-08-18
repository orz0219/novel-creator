//! Narrative Thread - Extended storyline with active progression tracking
//!
//! Design Doc 6: Different from simple Storyline.
//! NarrativeThread tracks current stage, recent progress, next step,
//! and which characters are actively participating.
//!
//! Example:
//!   Thread #001: "王家追杀主角"
//!   Status: Active
//!   Current Stage: "追踪"
//!   Recent Progress: "王家发现主角行踪"
//!   Next Step: "王家派出筑基修士"
//!   Participants: [林凡, 王家家主, 王家护卫]

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Thread status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum NarrativeThreadStatus {
    /// Active
    Active,
    /// Paused (temporarily inactive)
    Paused,
    /// Resolved (goal achieved)
    Resolved,
    /// Abandoned
    Abandoned,
}

/// Thread importance
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum NarrativeThreadImportance {
    /// Main thread
    Main,
    /// Important subplot
    Important,
    /// Normal subplot
    Normal,
    /// Minor subplot
    Minor,
}

/// Narrative Thread - active story progression tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NarrativeThread {
    pub id: Uuid,
    pub project_id: Uuid,
    /// Optional link to Storyline
    pub storyline_id: Option<Uuid>,
    pub name: String,
    pub description: Option<String>,
    pub status: NarrativeThreadStatus,
    pub importance: NarrativeThreadImportance,
    /// Current stage description
    pub current_stage: Option<String>,
    /// Recent progress description
    pub recent_progress: Option<String>,
    /// Next planned step
    pub next_step: Option<String>,
    /// Goal of this thread
    pub goal: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Thread participant - entity actively involved in a thread
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NarrativeThreadParticipant {
    pub id: Uuid,
    pub thread_id: Uuid,
    pub entity_id: Uuid,
    /// Role in the thread (e.g., "protagonist", "antagonist", "support")
    pub role: Option<String>,
    pub created_at: DateTime<Utc>,
}
