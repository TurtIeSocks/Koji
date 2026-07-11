//! Dragonite's V2 response envelope (`routes/v2_envelope.go`).
//!
//! Dragonite's `/v2/areas/*` API does **not** use the JSend `success`/`fail`
//! shape — it wraps every response in `V2Response[T]`:
//!
//! ```jsonc
//! // success
//! { "status": "ok", "data": <T>, "meta": { … }? }
//! // error
//! { "status": "error", "error": { "code": "…", "message": "…", "field": "…"? } }
//! ```
//!
//! `status` is the string enum `"ok" | "error"`. Success responses omit `error`
//! and carry `data` (unknown members like a list `meta` are ignored); error responses omit
//! `data` and carry `error`. This module decodes that envelope and collapses it
//! to a `Result`.

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::error::DragoniteError;

/// The V2 response envelope, discriminated by the `status` string.
#[derive(Debug, Clone, Deserialize)]
pub struct V2Envelope<T> {
    /// `"ok"` or `"error"`.
    pub status: String,
    /// Present on `ok` responses.
    #[serde(default = "Option::default")]
    pub data: Option<T>,
    /// Present on `error` responses.
    #[serde(default)]
    pub error: Option<V2ApiError>,
}

/// The on-the-wire V2 error object (`error{code,message,field}`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct V2ApiError {
    /// Stable machine-readable error code (e.g. `"not_found"`, `"invalid_json"`).
    pub code: String,
    /// Human-readable message.
    pub message: String,
    /// The offending request field, for per-field validation errors.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
}

impl<T> V2Envelope<T> {
    /// Collapse the envelope into `data` or a [`DragoniteError::Api`].
    ///
    /// - `status == "ok"` → `Ok(data)`; a missing `data` is a decode error
    ///   (an `ok` envelope must carry its payload).
    /// - `status == "error"` → `Err(Api { code, message, field })`.
    /// - anything else → decode error (unknown status).
    ///
    /// (The pagination `meta` half was deleted: the client's only method is
    /// `patch_area` and every reader discarded it. Reintroduce alongside the
    /// first real list endpoint.)
    pub fn into_result(self) -> Result<T, DragoniteError> {
        match self.status.as_str() {
            "ok" => match self.data {
                Some(data) => Ok(data),
                None => Err(DragoniteError::Decode(
                    "v2 `ok` envelope missing `data`".to_string(),
                )),
            },
            "error" => {
                let e = self.error.unwrap_or(V2ApiError {
                    code: "unknown".to_string(),
                    message: "v2 `error` envelope missing `error` object".to_string(),
                    field: None,
                });
                Err(DragoniteError::Api {
                    code: Some(e.code),
                    message: e.message,
                    field: e.field,
                })
            }
            other => Err(DragoniteError::Decode(format!(
                "v2 envelope has unknown status {other:?}"
            ))),
        }
    }
}

/// Decode a raw body into `V2Envelope<T>` and collapse it.
/// A body that is not a valid envelope maps to [`DragoniteError::Decode`].
pub fn parse_v2<T: DeserializeOwned>(body: &[u8]) -> Result<T, DragoniteError> {
    let envelope: V2Envelope<T> =
        serde_json::from_slice(body).map_err(|e| DragoniteError::Decode(e.to_string()))?;
    envelope.into_result()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Deserialize, PartialEq)]
    struct Payload {
        id: i64,
        name: String,
    }

    #[test]
    fn ok_envelope_extracts_data() {
        let body = br#"{"status":"ok","data":{"id":7,"name":"alpha"}}"#;
        let got: Payload = parse_v2(body).expect("ok should parse");
        assert_eq!(
            got,
            Payload {
                id: 7,
                name: "alpha".into()
            }
        );
    }


    #[test]
    fn error_envelope_maps_to_api_error_with_code_and_field() {
        let body =
            br#"{"status":"error","error":{"code":"invalid_path","message":"bad id","field":"area_id"}}"#;
        let err = parse_v2::<Payload>(body).expect_err("error should fail");
        match err {
            DragoniteError::Api {
                code,
                message,
                field,
            } => {
                assert_eq!(code.as_deref(), Some("invalid_path"));
                assert_eq!(message, "bad id");
                assert_eq!(field.as_deref(), Some("area_id"));
            }
            other => panic!("expected Api error, got {other:?}"),
        }
    }

    #[test]
    fn ok_without_data_is_decode_error() {
        let body = br#"{"status":"ok"}"#;
        let err = parse_v2::<Payload>(body).expect_err("ok-without-data should decode-fail");
        assert!(matches!(err, DragoniteError::Decode(_)));
    }

    #[test]
    fn unknown_status_is_decode_error() {
        let body = br#"{"status":"success","data":{"id":1,"name":"x"}}"#;
        let err = parse_v2::<Payload>(body).expect_err("non-v2 status should decode-fail");
        assert!(matches!(err, DragoniteError::Decode(_)));
    }
}
