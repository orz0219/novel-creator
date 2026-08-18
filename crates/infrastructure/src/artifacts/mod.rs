//! Artifact Storage for LLM outputs and large text

pub mod storage;
pub mod types;

pub use storage::ArtifactStorage;
pub use types::{Artifact, ArtifactType};
