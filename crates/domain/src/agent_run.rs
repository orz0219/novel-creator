//! Agent Run - Agent execution history
//!
//! Records every agent invocation for debugging, auditing, and recovery.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AgentRunStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

/// Agent execution record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRun {
    pub id: Uuid,
    pub project_id: Uuid,
    pub agent_type: String,
    pub task_type: String,
    pub status: AgentRunStatus,
    pub input: serde_json::Value,
    pub output: serde_json::Value,
    pub error_message: Option<String>,
    pub model: Option<String>,
    pub provider: Option<String>,
    pub context_snapshot_id: Option<Uuid>,
    pub tokens_used: Option<i32>,
    pub duration_ms: Option<i64>,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub metadata: serde_json::Value,
}
