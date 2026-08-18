//! Artifact types

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Artifact type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ArtifactType {
    /// LLM response
    LlmResponse,
    /// Context snapshot
    ContextSnapshot,
    /// Draft content
    Draft,
    /// Prompt template
    Prompt,
    /// Generated image
    Image,
    /// Other
    Other(String),
}

/// Artifact - stored content outside DuckDB
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artifact {
    pub id: Uuid,
    pub project_id: Uuid,
    pub artifact_type: ArtifactType,
    pub content_hash: String,
    pub storage_path: String,
    pub mime_type: String,
    pub size_bytes: u64,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
}
