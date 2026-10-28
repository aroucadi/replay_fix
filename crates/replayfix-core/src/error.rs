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

    #[error("Patch error: {0}")]
    Patch(String),

    #[error("Worktree isolation error: {0}")]
    Worktree(String),

    #[error("Browser bridge error: {0}")]
    Browser(String),

    #[error("CDP or bridge command timeout: {0}")]
    CdpTimeout(String),

    #[error("Ambiguous or ungrounded locator: {0}")]
    Grounding(String),

    #[error("Model API error: {0}")]
    ModelApiError(String),

    #[error("Patch application error: {0}")]
    PatchApplicationError(String),

    #[error("Repair budget exceeded: {0}")]
    RepairBudgetExceeded(String),

    #[error("Syntax validation failed: {0}")]
    SyntaxValidationError(String),

    #[error("Session checkpoint error: {0}")]
    Checkpoint(String),

    #[error("Regression parity error: {0}")]
    Regression(String),
}

pub type Result<T> = std::result::Result<T, CoreError>;
