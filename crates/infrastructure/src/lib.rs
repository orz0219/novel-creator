//! Infrastructure Layer - LLM abstraction, artifact storage, observability
//!
//! NOTE: The database module was previously disabled due to DuckDB-specific compilation errors.
//! It will be rebuilt for PostgreSQL in a future phase if needed. The primary database layer
//! lives in crates/db with sqlx + PgPool.

// pub mod database;  // TODO: rebuild for PostgreSQL if needed
pub mod llm;
pub mod artifacts;
pub mod observability;
pub mod error;

// Re-export commonly used types
pub use error::{NovelError, DomainError, ApplicationError, InfrastructureError};
