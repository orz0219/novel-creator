//! Novel State Snapshot - Full novel state at a point in time
//!
//! Design Doc 6: Different from ContextSnapshot (which is per AI call).
//! NovelStateSnapshot captures the entire novel's macro state:
//! - Story time
//! - World summary
//! - Main character state
//! - Active threads count
//! - Unresolved foreshadowing count
//! - Current volume/arc

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Novel State Snapshot - complete novel state at a point in time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NovelStateSnapshot {
    pub id: Uuid,
    pub project_id: Uuid,
    /// The scene this snapshot was taken after
    pub scene_id: Option<Uuid>,
    /// In-story time (e.g., "Day 381")
    pub story_time: Option<String>,
    /// World state summary
    pub world_summary: Option<String>,
    /// Main character state summary
    pub main_character_state: Option<String>,
    /// Current location name
    pub current_location: Option<String>,
    /// Number of active narrative threads
    pub active_threads_count: i32,
    /// Number of unresolved foreshadowing
    pub unresolved_foreshadows_count: i32,
    /// Number of known characters
    pub known_characters_count: i32,
    /// Number of known locations
    pub known_locations_count: i32,
    /// Current volume ID
    pub current_volume_id: Option<Uuid>,
    /// Current arc ID
    pub current_arc_id: Option<Uuid>,
    /// Full structured state data
    pub state_data: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

impl NovelStateSnapshot {
    pub fn new(project_id: Uuid) -> Self {
        Self {
            id: Uuid::new_v4(),
            project_id,
            scene_id: None,
            story_time: None,
            world_summary: None,
            main_character_state: None,
            current_location: None,
            active_threads_count: 0,
            unresolved_foreshadows_count: 0,
            known_characters_count: 0,
            known_locations_count: 0,
            current_volume_id: None,
            current_arc_id: None,
            state_data: serde_json::json!({}),
            created_at: Utc::now(),
        }
    }
}
