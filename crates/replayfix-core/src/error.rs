use thiserror::Error;

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("Platform I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Command execution error: {0}")]
    Execution(String),

    #[error("Environment doctor error: {0}")]
    Environment(String),

    #[error("Task manifest validation error: {0}")]
    Manifest(String),

    #[error("Security boundary violation: {0}")]
    SecurityViolation(String),

    #[error("Verification failed: {0}")]
    Verification(String),
}

pub type Result<T> = std::result::Result<T, CoreError>;
