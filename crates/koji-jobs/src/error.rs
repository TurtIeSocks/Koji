//! Error types for the job queue (design spec §11).
//!
//! - [`JobError`] is the *handler-side* error. A handler's `run` returns it; the
//!   worker serializes it to `job.error` and records a stable v2 error code
//!   (`validation_error`, `internal_error`, …) via [`JobError::code`].
//! - [`AwaitError`] is what the sync bridge sees from `await_result` — note that
//!   a timeout is **not** a job failure (§11: "`await` timeout → 504; never
//!   marks the job failed").
//! - [`EnqueueError`] wraps failures of `enqueue_or_attach`.

use thiserror::Error;

/// Handler-side failure. Returned by [`crate::JobHandler::run`] and persisted to
/// `job.error`.
///
/// The variant maps to a stable v2 error code via [`JobError::code`], which the
/// sync bridge turns into a JSend `error.code` + the right HTTP status.
#[derive(Debug, Error)]
pub enum JobError {
    /// Bad input / unprocessable request → `validation_error`.
    #[error("validation error: {0}")]
    Validation(String),

    /// Unexpected internal failure → `internal_error`.
    #[error("internal error: {0}")]
    Internal(String),

    /// A handler-specific failure carrying its own stable code + message. Lets a
    /// concrete handler surface a domain code without growing this enum.
    #[error("{message}")]
    Custom {
        /// Stable, snake_case error code returned by [`JobError::code`].
        code: String,
        /// Human-readable message persisted to `job.error`.
        message: String,
    },
}

impl JobError {
    /// Stable v2 error code for this failure (snake_case, machine-readable).
    ///
    /// `Custom` returns its caller-supplied code verbatim.
    pub fn code(&self) -> &str {
        match self {
            JobError::Validation(_) => "validation_error",
            JobError::Internal(_) => "internal_error",
            JobError::Custom { code, .. } => code.as_str(),
        }
    }

    /// Convenience constructor for [`JobError::Validation`].
    pub fn validation(msg: impl Into<String>) -> Self {
        JobError::Validation(msg.into())
    }

    /// Convenience constructor for [`JobError::Internal`].
    pub fn internal(msg: impl Into<String>) -> Self {
        JobError::Internal(msg.into())
    }

    /// Convenience constructor for [`JobError::Custom`].
    pub fn custom(code: impl Into<String>, message: impl Into<String>) -> Self {
        JobError::Custom {
            code: code.into(),
            message: message.into(),
        }
    }
}

/// Failure of [`crate::JobQueue::await_result`].
///
/// `Timeout` is the sync-bridge's 290s→504 case: the job is *still running* and
/// will populate the grace window — it is never a job failure.
#[derive(Debug, Error)]
pub enum AwaitError {
    /// The wait deadline elapsed before the job reached a terminal state. The
    /// job continues running (spec §7).
    #[error("timed out waiting for job result")]
    Timeout,

    /// No job exists with the requested id.
    #[error("job not found")]
    NotFound,

    /// An underlying database error occurred while polling/loading the job.
    #[error("database error: {0}")]
    Db(#[from] sea_orm::DbErr),
}

/// Failure of [`crate::JobQueue::enqueue_or_attach`].
#[derive(Debug, Error)]
pub enum EnqueueError {
    /// The payload could not be serialized to JSON.
    #[error("failed to serialize job payload: {0}")]
    Serialize(#[from] serde_json::Error),

    /// An underlying database error occurred while inserting/coalescing.
    #[error("database error: {0}")]
    Db(#[from] sea_orm::DbErr),
}
