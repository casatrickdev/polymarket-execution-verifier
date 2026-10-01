use thiserror::Error;

use crate::domain::states::ExecutionState;

#[derive(Debug, Error)]
pub enum VerifierError {
    #[error("execution not found: {0}")]
    ExecutionNotFound(String),

    #[error("illegal state transition from {from} to {to}: {reason}")]
    IllegalTransition {
        from: ExecutionState,
        to: ExecutionState,
        reason: String,
    },

    #[error("duplicate event ignored: {event_id}")]
    DuplicateEvent { event_id: String },

    #[error("repository error: {0}")]
    Repository(#[from] RepositoryError),

    #[error("gateway error: {0}")]
    Gateway(String),

    #[error("reconciliation error: {0}")]
    Reconciliation(String),

    #[error("invalid data: {0}")]
    InvalidData(String),
}

#[derive(Debug, Error)]
pub enum RepositoryError {
    #[error("sqlite: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("not found: {0}")]
    NotFound(String),

    #[error("serialization: {0}")]
    Serialization(String),
}

pub type VerifierResult<T> = Result<T, VerifierError>;
