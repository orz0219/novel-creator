//! Artifact types

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ArtifactType {
    LlmResponse,
    ContextSnapshot,
    Draft,
    Prompt,
    Image,
    Other(String),
}

/// Artifact - stored content outside the database (large files on disk)
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
