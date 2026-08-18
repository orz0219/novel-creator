//! Unified Error Model

use thiserror::Error;

/// Domain errors - errors from business logic
#[derive(Error, Debug)]
pub enum DomainError {
    #[error("Entity not found: {0}")]
    EntityNotFound(String),

    #[error("World not found: {0}")]
    WorldNotFound(String),

    #[error("Project not found: {0}")]
    ProjectNotFound(String),

    #[error("Validation failed: {0}")]
    ValidationFailed(String),

    #[error("Canon violation: {0}")]
    CanonViolation(String),

    #[error("Version conflict: {0}")]
    VersionConflict(String),

    #[error("State transition invalid: {0}")]
    InvalidStateTransition(String),

    #[error("Unauthorized operation: {0}")]
    Unauthorized(String),

    #[error("Business rule violation: {0}")]
    BusinessRuleViolation(String),
}

/// Application errors - errors from application layer
#[derive(Error, Debug)]
pub enum ApplicationError {
    #[error("Command execution failed: {0}")]
    CommandExecutionFailed(String),

    #[error("Query failed: {0}")]
    QueryFailed(String),

    #[error("Service unavailable: {0}")]
    ServiceUnavailable(String),

    #[error("Rate limit exceeded")]
    RateLimitExceeded,

    #[error("Timeout: {0}")]
    Timeout(String),
}

/// Infrastructure errors - errors from infrastructure layer
#[derive(Error, Debug)]
pub enum InfrastructureError {
    #[error("Database error: {0}")]
    Database(#[from] duckdb::Error),

    #[error("LLM error: {0}")]
    Llm(String),

    #[error("Artifact storage error: {0}")]
    ArtifactStorage(String),

    #[error("Configuration error: {0}")]
    Configuration(String),

    #[error("Connection pool exhausted")]
    ConnectionPoolExhausted,

    #[error("Transaction failed: {0}")]
    TransactionFailed(String),

    #[error("Network error: {0}")]
    Network(String),
}

/// Unified error type
#[derive(Error, Debug)]
pub enum NovelError {
    #[error("Domain error: {0}")]
    Domain(#[from] DomainError),

    #[error("Application error: {0}")]
    Application(#[from] ApplicationError),

    #[error("Infrastructure error: {0}")]
    Infrastructure(#[from] InfrastructureError),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl From<anyhow::Error> for NovelError {
    fn from(err: anyhow::Error) -> Self {
        NovelError::Internal(err.to_string())
    }
}

/// Error codes for API responses
pub mod codes {
    pub const ENTITY_NOT_FOUND: &str = "ENTITY_NOT_FOUND";
    pub const WORLD_NOT_FOUND: &str = "WORLD_NOT_FOUND";
    pub const PROJECT_NOT_FOUND: &str = "PROJECT_NOT_FOUND";
    pub const VALIDATION_FAILED: &str = "VALIDATION_FAILED";
    pub const CANON_VIOLATION: &str = "CANON_VIOLATION";
    pub const VERSION_CONFLICT: &str = "VERSION_CONFLICT";
    pub const INVALID_STATE_TRANSITION: &str = "INVALID_STATE_TRANSITION";
    pub const UNAUTHORIZED: &str = "UNAUTHORIZED";
    pub const BUSINESS_RULE_VIOLATION: &str = "BUSINESS_RULE_VIOLATION";
    pub const COMMAND_EXECUTION_FAILED: &str = "COMMAND_EXECUTION_FAILED";
    pub const QUERY_FAILED: &str = "QUERY_FAILED";
    pub const SERVICE_UNAVAILABLE: &str = "SERVICE_UNAVAILABLE";
    pub const RATE_LIMIT_EXCEEDED: &str = "RATE_LIMIT_EXCEEDED";
    pub const TIMEOUT: &str = "TIMEOUT";
    pub const DATABASE_ERROR: &str = "DATABASE_ERROR";
    pub const LLM_ERROR: &str = "LLM_ERROR";
    pub const ARTIFACT_STORAGE_ERROR: &str = "ARTIFACT_STORAGE_ERROR";
    pub const CONFIGURATION_ERROR: &str = "CONFIGURATION_ERROR";
    pub const CONNECTION_POOL_EXHAUSTED: &str = "CONNECTION_POOL_EXHAUSTED";
    pub const TRANSACTION_FAILED: &str = "TRANSACTION_FAILED";
    pub const NETWORK_ERROR: &str = "NETWORK_ERROR";
    pub const INTERNAL_ERROR: &str = "INTERNAL_ERROR";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_domain_error() {
        let err = DomainError::EntityNotFound("test".to_string());
        assert!(err.to_string().contains("Entity not found"));
    }

    #[test]
    fn test_application_error() {
        let err = ApplicationError::CommandExecutionFailed("test".to_string());
        assert!(err.to_string().contains("Command execution failed"));
    }

    #[test]
    fn test_infrastructure_error() {
        let err = InfrastructureError::Llm("test".to_string());
        assert!(err.to_string().contains("LLM error"));
    }

    #[test]
    fn test_novel_error() {
        let err = NovelError::Domain(DomainError::EntityNotFound("test".to_string()));
        assert!(err.to_string().contains("Domain error"));
    }
}
