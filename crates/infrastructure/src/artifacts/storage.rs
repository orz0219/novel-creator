//! Artifact Storage implementation

use anyhow::Result;
use sha2::{Sha256, Digest};
use std::fs;
use std::path::PathBuf;
use tracing::info;

use super::types::{Artifact, ArtifactType};

/// Artifact storage - stores content outside DuckDB
pub struct ArtifactStorage {
    base_path: PathBuf,
}

impl ArtifactStorage {
    /// Create a new artifact storage
    pub fn new(base_path: &str) -> Self {
        let path = PathBuf::from(base_path);
        fs::create_dir_all(&path).expect("Failed to create artifact storage directory");
        Self { base_path: path }
    }

    /// Store an artifact
    pub fn store(&self, project_id: uuid::Uuid, artifact_type: ArtifactType, content: &[u8], mime_type: &str) -> Result<Artifact> {
        let id = uuid::Uuid::new_v4();
        let content_hash = self.calculate_hash(content);
        
        // Create project directory
        let project_dir = self.base_path.join(project_id.to_string());
        fs::create_dir_all(&project_dir)?;

        // Create artifact file
        let filename = format!("{}.{}", id, self.get_extension(mime_type));
        let file_path = project_dir.join(&filename);
        fs::write(&file_path, content)?;

        let artifact = Artifact {
            id,
            project_id,
            artifact_type,
            content_hash,
            storage_path: file_path.to_string_lossy().to_string(),
            mime_type: mime_type.to_string(),
            size_bytes: content.len() as u64,
            metadata: serde_json::json!({}),
            created_at: Utc::now(),
        };

        info!("Artifact stored: {} ({})", id, mime_type);
        Ok(artifact)
    }

    /// Retrieve an artifact
    pub fn retrieve(&self, artifact: &Artifact) -> Result<Vec<u8>> {
        let content = fs::read(&artifact.storage_path)?;
        
        // Verify hash
        let hash = self.calculate_hash(&content);
        if hash != artifact.content_hash {
            return Err(anyhow::anyhow!("Artifact hash mismatch"));
        }

        Ok(content)
    }

    /// Delete an artifact
    pub fn delete(&self, artifact: &Artifact) -> Result<()> {
        if std::path::Path::new(&artifact.storage_path).exists() {
            fs::remove_file(&artifact.storage_path)?;
        }
        Ok(())
    }

    /// Calculate content hash
    fn calculate_hash(&self, content: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(content);
        format!("{:x}", hasher.finalize())
    }

    /// Get file extension from mime type
    fn get_extension(&self, mime_type: &str) -> &str {
        match mime_type {
            "text/plain" => "txt",
            "application/json" => "json",
            "image/png" => "png",
            "image/jpeg" => "jpg",
            "text/html" => "html",
            _ => "bin",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_artifact_storage() {
        let temp_dir = tempdir().unwrap();
        let storage = ArtifactStorage::new(temp_dir.path().to_str().unwrap());

        let project_id = uuid::Uuid::new_v4();
        let content = b"Hello, World!";

        // Store artifact
        let artifact = storage.store(
            project_id,
            ArtifactType::LlmResponse,
            content,
            "text/plain",
        ).unwrap();

        assert_eq!(artifact.size_bytes, 13);
        assert_eq!(artifact.mime_type, "text/plain");

        // Retrieve artifact
        let retrieved = storage.retrieve(&artifact).unwrap();
        assert_eq!(retrieved, content);

        // Delete artifact
        storage.delete(&artifact).unwrap();
    }
}
