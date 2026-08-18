//! Infrastructure Layer - DuckDB implementations, LLM abstraction, artifact storage, observability

pub mod database;
pub mod llm;
pub mod artifacts;
pub mod observability;
pub mod error;

// Re-export commonly used types
pub use error::{NovelError, DomainError, ApplicationError, InfrastructureError};
