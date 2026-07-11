//! Error type for the Dragonite client.

use thiserror::Error;

/// Errors produced by [`crate::DragoniteClient`] and the V2 envelope → `Result`
/// conversion.
#[derive(Debug, Error)]
pub enum DragoniteError {
    /// Transport-level failure (connection, timeout, non-decodable body at the
    /// reqwest layer, URL build error, etc.).
    #[error("dragonite http error: {0}")]
    Http(#[from] reqwest::Error),

    /// Dragonite returned a V2 `error` envelope (`{"status":"error","error":{…}}`).
    /// `code` is the stable machine-readable error-code string from
    /// [`V2ApiError`](crate::envelope::V2ApiError); `field` names the offending
    /// request field when the error is a per-field validation failure.
    #[error("dragonite api error: {message}{}{}",
        .code.as_ref().map(|c| format!(" (code: {c})")).unwrap_or_default(),
        .field.as_ref().map(|f| format!(" [field: {f}]")).unwrap_or_default())]
    Api {
        code: Option<String>,
        message: String,
        field: Option<String>,
    },

    /// A non-2xx response that carried no parseable V2 envelope: the raw HTTP
    /// status plus body. Typed (not a stringly `http_<code>` Api code) so the
    /// dispatcher-side backoff mapping can match on it directly.
    #[error("dragonite http {status}: {body}")]
    HttpStatus { status: u16, body: String },

    /// A response body did not match the expected shape — not a valid V2
    /// envelope, an `ok` envelope missing its `data`, or a `data` payload that
    /// could not be deserialized into the target type.
    #[error("dragonite decode error: {0}")]
    Decode(String),
}
