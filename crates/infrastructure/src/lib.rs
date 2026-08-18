//! Infrastructure Layer - Database, LLM abstraction, artifact storage, observability
//!
//! Database access uses PostgreSQL via sqlx::PgPool.

pub mod database;
pub mod llm;
pub mod artifacts;
pub mod observability;
pub mod error;

pub use error::{NovelError, DomainError, ApplicationError, InfrastructureError};
