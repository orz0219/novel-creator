//! System Event - System event log
//!
//! Records all significant system events for auditing and history.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// System event entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemEvent {
    pub id: Uuid,
    pub project_id: Option<Uuid>,
    pub event_type: String,
    pub entity_type: Option<String>,
    pub entity_id: Option<Uuid>,
    pub actor: String,
    pub description: String,
    pub old_value: Option<serde_json::Value>,
    pub new_value: Option<serde_json::Value>,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
}
