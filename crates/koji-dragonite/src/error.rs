//! Error type for the Dragonite client.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Errors produced by [`crate::DragoniteClient`].
#[derive(Debug, Error)]
pub enum DragoniteError {
    /// Transport-level failure (connection, timeout, non-decodable body at the
    /// reqwest layer, URL build error, etc.).
    #[error("dragonite http error: {0}")]
    Http(#[from] reqwest::Error),

    /// A non-2xx response: the raw HTTP status, plus the problem document's
    /// `detail` and `errors` when the body was one. `body` is the raw reply text
    /// (empty when Dragonite sent none). Typed so the dispatcher-side backoff
    /// mapping can match on `status` directly.
    #[error("dragonite http {status}: {}{}",
        .detail.as_deref().unwrap_or(.body),
        format_problem_errors(.errors))]
    HttpStatus {
        status: u16,
        detail: Option<String>,
        errors: Vec<ProblemError>,
        body: String,
    },

    /// A 2xx response whose body could not be deserialized into the target type.
    #[error("dragonite decode error: {0}")]
    Decode(String),
}

/// One entry of a problem document's `errors` array
/// (`{"message":…,"location":"body.enabled","value":…}`).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ProblemError {
    /// What is wrong, e.g. `"expected boolean"`.
    #[serde(default)]
    pub message: Option<String>,
    /// Where it is wrong, e.g. `"body.enabled"`.
    #[serde(default)]
    pub location: Option<String>,
    /// The offending value as the request carried it.
    #[serde(default)]
    pub value: Option<serde_json::Value>,
}

/// The parts of Dragonite's RFC 9457 problem document Koji reads. The
/// document's own `status` is a number and is deliberately not read: the HTTP
/// status line is the source of truth.
#[derive(Debug, Default, Deserialize)]
pub(crate) struct ProblemDocument {
    #[serde(default)]
    pub detail: Option<String>,
    #[serde(default)]
    pub errors: Vec<ProblemError>,
}

/// ` (location: message; …)` for the `Display` of [`DragoniteError::HttpStatus`],
/// or nothing when there are no entries.
fn format_problem_errors(errors: &[ProblemError]) -> String {
    if errors.is_empty() {
        return String::new();
    }
    let parts: Vec<String> = errors
        .iter()
        .map(|e| match (e.location.as_deref(), e.message.as_deref()) {
            (Some(l), Some(m)) => format!("{l}: {m}"),
            (Some(x), None) | (None, Some(x)) => x.to_string(),
            (None, None) => String::new(),
        })
        .collect();
    format!(" ({})", parts.join("; "))
}
