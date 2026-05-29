//! Generic [JSend](https://github.com/omniti-labs/jsend) envelope.
//!
//! This is the SOUND, reusable part of the crate: JSend is a published spec and
//! Koji V2 itself adopts it (architecture §6/§7). The three-variant shape below
//! is the JSend standard and is *not* speculative.

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::error::DragoniteError;

/// A JSend response envelope, discriminated by the `status` field.
///
/// - `success` — the request succeeded; `data` holds the payload.
/// - `fail` — the request was rejected due to invalid input; `data` holds a
///   map of validation problems (kept as raw JSON — its shape is caller- and
///   endpoint-specific).
/// - `error` — the server hit an error processing the request; `message` is
///   required, `code`/`data` are optional.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "lowercase")]
pub enum JSend<T> {
    Success {
        data: T,
    },
    Fail {
        data: serde_json::Value,
    },
    Error {
        message: String,
        #[serde(skip_serializing_if = "Option::is_none", default)]
        code: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none", default)]
        data: Option<serde_json::Value>,
    },
}

impl<T> JSend<T> {
    /// Collapse the envelope into a `Result`, mapping both `fail` and `error`
    /// onto [`DragoniteError::Api`].
    ///
    /// A `fail` envelope has no machine code, so its validation `data` is
    /// rendered into the message and `code` is `None`. An `error` envelope's
    /// `message`/`code` pass through directly.
    pub fn into_result(self) -> Result<T, DragoniteError> {
        match self {
            JSend::Success { data } => Ok(data),
            JSend::Fail { data } => Err(DragoniteError::Api {
                message: format!("request failed: {data}"),
                code: None,
            }),
            JSend::Error { message, code, .. } => Err(DragoniteError::Api { message, code }),
        }
    }
}

/// Deserialize a raw JSON body into `JSend<T>` and immediately collapse it to a
/// `Result`. Decode failures (body that is not even a valid JSend envelope) map
/// to [`DragoniteError::Decode`].
pub fn parse_jsend<T: DeserializeOwned>(body: &[u8]) -> Result<T, DragoniteError> {
    let envelope: JSend<T> =
        serde_json::from_slice(body).map_err(|e| DragoniteError::Decode(e.to_string()))?;
    envelope.into_result()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct Payload {
        id: u32,
        name: String,
    }

    #[test]
    fn deserializes_success_and_extracts_data() {
        let body = br#"{"status":"success","data":{"id":7,"name":"alpha"}}"#;
        let got: Payload = parse_jsend(body).expect("success should parse");
        assert_eq!(
            got,
            Payload {
                id: 7,
                name: "alpha".into()
            }
        );
    }

    #[test]
    fn success_round_trips_through_serialize() {
        let env = JSend::Success {
            data: Payload {
                id: 1,
                name: "x".into(),
            },
        };
        let json = serde_json::to_value(&env).unwrap();
        assert_eq!(json["status"], "success");
        assert_eq!(json["data"]["id"], 1);
        assert_eq!(json["data"]["name"], "x");
    }

    #[test]
    fn error_envelope_maps_to_api_error_with_code() {
        let body = br#"{"status":"error","message":"boom","code":"E_BOOM"}"#;
        let err = parse_jsend::<Payload>(body).expect_err("error should fail");
        match err {
            DragoniteError::Api { message, code } => {
                assert_eq!(message, "boom");
                assert_eq!(code.as_deref(), Some("E_BOOM"));
            }
            other => panic!("expected Api error, got {other:?}"),
        }
    }

    #[test]
    fn fail_envelope_maps_to_api_error_without_code() {
        let body = br#"{"status":"fail","data":{"name":"required"}}"#;
        let err = parse_jsend::<Payload>(body).expect_err("fail should error");
        match err {
            DragoniteError::Api { message, code } => {
                assert!(message.contains("required"), "message was: {message}");
                assert!(code.is_none());
            }
            other => panic!("expected Api error, got {other:?}"),
        }
    }

    #[test]
    fn non_envelope_body_maps_to_decode_error() {
        let body = br#"{"totally":"unexpected"}"#;
        let err = parse_jsend::<Payload>(body).expect_err("garbage should decode-fail");
        assert!(matches!(err, DragoniteError::Decode(_)));
    }
}
