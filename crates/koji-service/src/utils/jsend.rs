//! The v2 [JSend](https://github.com/omniti-labs/jsend) response envelope.
//!
//! v2 handlers return a `JSend<T>` serialized as the response body alongside a
//! semantic HTTP status. The three arms mirror the JSend spec (and
//! `koji_dragonite::jsend`, which Koji V2 may later share):
//!
//! - `success` — the request worked; `data` carries the payload.
//! - `fail`    — the request was rejected for a client-side reason (validation);
//!   `data` carries a machine-readable description of what was wrong.
//! - `error`   — the request failed processing server-side; `message` is required,
//!   `code` is an optional stable machine code, `data` optional extra context.
//!
//! The `status` field is the serde tag, so the wire form is
//! `{"status":"success","data":…}` etc.

use actix_web::{HttpResponse, http::StatusCode};
use serde::Serialize;
use serde_json::Value;

/// The JSend envelope. Generic over the success `data` type; `fail`/`error`
/// carry untyped JSON because their shape is per-error, not per-endpoint.
#[derive(Debug, Serialize)]
#[serde(tag = "status", rename_all = "lowercase")]
pub enum JSend<T> {
    /// All went well; `data` is the endpoint payload.
    Success {
        /// The successful payload.
        data: T,
    },
    /// Rejected for a client-side reason; `data` describes what was wrong.
    Fail {
        /// Machine-readable description of the rejection.
        data: Value,
    },
    /// Processing failed server-side.
    Error {
        /// Human-readable error message (required by JSend).
        message: String,
        /// Stable, machine-readable code (e.g. `validation_error`).
        #[serde(skip_serializing_if = "Option::is_none")]
        code: Option<String>,
        /// Optional extra context.
        #[serde(skip_serializing_if = "Option::is_none")]
        data: Option<Value>,
    },
}

impl<T: Serialize> JSend<T> {
    /// Build a `200 OK` `success` response carrying `data`.
    pub fn success(data: T) -> HttpResponse {
        Self::success_with_status(StatusCode::OK, data)
    }

    /// Build a `success` response with an explicit status (e.g. `201`/`202`).
    pub fn success_with_status(status: StatusCode, data: T) -> HttpResponse {
        HttpResponse::build(status).json(JSend::Success { data })
    }
}

impl JSend<()> {
    /// Build an `error` response with the given status, message, and optional
    /// machine code / context.
    pub fn error(
        status: StatusCode,
        message: impl Into<String>,
        code: Option<String>,
        data: Option<Value>,
    ) -> HttpResponse {
        HttpResponse::build(status).json(JSend::<()>::Error {
            message: message.into(),
            code,
            data,
        })
    }

    /// Build a `fail` response (client-side rejection) with the given status and
    /// `data` describing the problem. Defaults to `400 Bad Request` semantics —
    /// pass the status the caller wants.
    pub fn fail(status: StatusCode, data: Value) -> HttpResponse {
        HttpResponse::build(status).json(JSend::<()>::Fail { data })
    }
}
