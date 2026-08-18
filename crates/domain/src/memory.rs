//! Memory - Cross-session memory system
//!
//! Memories persist knowledge across sessions for agents and users.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MemoryType {
    World, Character, Plot, Preference, Decision, Event, Agent, Project, General,
}

impl MemoryType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::World => "world", Self::Character => "character", Self::Plot => "plot",
            Self::Preference => "preference", Self::Decision => "decision", Self::Event => "event",
            Self::Agent => "agent", Self::Project => "project", Self::General => "general",
        }
    }
}

/// Cross-session memory entry (named SessionMemory to avoid conflict with character_mind::Memory)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMemory {
    pub id: Uuid,
    pub project_id: Uuid,
    pub memory_type: MemoryType,
    pub content: String,
    pub importance: f64,
    pub source: Option<String>,
    pub embedding_vector_id: Option<String>,
    pub metadata: serde_json::Value,
    pub access_count: i32,
    pub last_accessed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
