//! Error type for the Dragonite client.

use thiserror::Error;

/// Errors produced by [`crate::DragoniteClient`] and the JSend → `Result`
/// conversion.
#[derive(Debug, Error)]
pub enum DragoniteError {
    /// Transport-level failure (connection, timeout, non-decodable body at the
    /// reqwest layer, URL build error, etc.).
    #[error("dragonite http error: {0}")]
    Http(#[from] reqwest::Error),

    /// Dragonite returned a JSend `fail` or `error` envelope. `code` is the
    /// optional machine-readable error code from an `error` envelope (`fail`
    /// envelopes carry no code).
    #[error("dragonite api error: {message}{}", .code.as_ref().map(|c| format!(" (code: {c})")).unwrap_or_default())]
    Api {
        message: String,
        code: Option<String>,
    },

    /// A 2xx body did not match the expected shape / could not be deserialized
    /// into the JSend envelope or its `data` payload.
    #[error("dragonite decode error: {0}")]
    Decode(String),
}
