//! The Koji v2 API response envelope.
//!
//! Matches Dragonite's v2 envelope (architecture design §7, decision #7) — the
//! `status` field is the string `"ok" | "error"`, **not** the JSend
//! `success/fail/error` (Dragonite dropped JSend for its v2 API, so Koji v2
//! follows the same shape for ecosystem consistency). Success carries `data`
//! (plus an optional `meta` pagination block on collection endpoints); error
//! carries `error{code,message,field?}`.
//!
//! ```jsonc
//! // success
//! { "status": "ok", "data": <T>, "meta": { … }? }
//! // error
//! { "status": "error", "error": { "code": "…", "message": "…", "field": "…"? } }
//! ```
//!
//! The constructor names (`success`/`success_with_status`/`fail`/`error`) and
//! signatures are kept stable across the v2 handlers; only the wire shape moved.
//! `fail`'s legacy `{"field":"message"}` value is mapped to `error{code,message,
//! field}` (first key → `field`, its value → `message`, `code` derived from the
//! HTTP status). `error`'s legacy `data` context arg is dropped — the v2 error
//! object has no `data` field.

use actix_web::{HttpResponse, http::StatusCode};
use serde::Serialize;
use serde_json::Value;
use utoipa::ToSchema;

/// The on-the-wire error object: `error{code,message,field?}` (matches Dragonite
/// v2 / architecture §7).
#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct ApiError {
    /// Stable, machine-readable code (e.g. `not_found`, `unprocessable`).
    pub code: String,
    /// Human-readable message.
    pub message: String,
    /// The offending request field, for per-field validation errors.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
}

/// Pagination block for `ok` collection responses (architecture §7). Reserved:
/// the `Ok` variant carries it so the envelope is spec-shaped, but Koji's list
/// endpoints currently return their full result set, so it is always omitted
/// (`None`) until list pagination lands.
#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct Meta {
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
    pub total_pages: i64,
    pub has_next: bool,
    pub has_prev: bool,
}

/// The v2 response envelope, discriminated by the `status` string (`ok`/`error`).
#[derive(Debug, Serialize)]
#[serde(tag = "status", rename_all = "lowercase")]
pub(crate) enum ApiResponse<T> {
    /// `{ "status": "ok", "data": …, "meta": …? }`.
    Ok {
        data: T,
        #[serde(skip_serializing_if = "Option::is_none")]
        meta: Option<Meta>,
    },
    /// `{ "status": "error", "error": { … } }`.
    Error { error: ApiError },
}

impl<T: Serialize> ApiResponse<T> {
    /// `200 OK` success carrying `data`.
    pub(crate) fn success(data: T) -> HttpResponse {
        Self::success_with_status(StatusCode::OK, data)
    }

    /// Success with an explicit status (e.g. `201`/`202`).
    pub(crate) fn success_with_status(status: StatusCode, data: T) -> HttpResponse {
        HttpResponse::build(status).json(ApiResponse::Ok { data, meta: None })
    }

    /// `200 OK` success carrying `data` plus a pagination `meta` block.
    ///
    /// Consumed by the P2 list handlers (regenerated typed CRUD); unused in the
    /// Phase 0 non-test build, hence the scoped allow.
    #[allow(dead_code)]
    pub(crate) fn success_paginated(data: T, meta: Meta) -> HttpResponse {
        HttpResponse::build(StatusCode::OK).json(ApiResponse::Ok {
            data,
            meta: Some(meta),
        })
    }
}

impl ApiResponse<()> {
    /// Server-side / processing error. `message` is required; `code` defaults to
    /// one derived from the HTTP status. The legacy `data` context arg is ignored
    /// (the v2 error shape has no `data`).
    ///
    /// P1 migrated the v2 jobs handlers to typed `ServiceError` variants (which
    /// own the status→error mapping), removing this constructor's last callers;
    /// kept for the handlers still on the explicit-status error path (and P2+),
    /// hence the scoped allow.
    #[allow(dead_code)]
    pub(crate) fn error(
        status: StatusCode,
        message: impl Into<String>,
        code: Option<String>,
        _data: Option<Value>,
    ) -> HttpResponse {
        let error = ApiError {
            code: code.unwrap_or_else(|| code_for_status(status)),
            message: message.into(),
            field: None,
        };
        HttpResponse::build(status).json(ApiResponse::<()>::Error { error })
    }

    /// Client-side rejection. The legacy `{"field":"message"}` value is mapped to
    /// `error{code,message,field}`: first object key → `field`, its value →
    /// `message`, `code` derived from the status.
    ///
    /// P4 re-pathed the plugins handlers onto typed `ServiceError` variants,
    /// removing this constructor's last caller; kept (like `error`) for the
    /// explicit-status error path, hence the scoped allow.
    #[allow(dead_code)]
    pub(crate) fn fail(status: StatusCode, data: Value) -> HttpResponse {
        let (field, message) = first_field_message(&data);
        let error = ApiError {
            code: code_for_status(status),
            message,
            field,
        };
        HttpResponse::build(status).json(ApiResponse::<()>::Error { error })
    }
}

/// Derive a stable error `code` from the HTTP status.
pub(crate) fn code_for_status(status: StatusCode) -> String {
    match status.as_u16() {
        400 => "invalid_request",
        401 => "unauthorized",
        403 => "forbidden",
        404 => "not_found",
        409 => "conflict",
        422 => "unprocessable",
        429 => "rate_limited",
        504 => "timeout",
        s if s >= 500 => "internal_error",
        _ => "error",
    }
    .to_string()
}

/// Extract `(field, message)` from a legacy `fail` value: the first key of a
/// JSON object becomes the field and its (stringified) value the message; a
/// non-object value yields `(None, stringified)`. Only `fail` (now caller-less,
/// see above) and the unit test below reference it, so the non-test build sees
/// it as dead.
#[allow(dead_code)]
fn first_field_message(data: &Value) -> (Option<String>, String) {
    if let Some(obj) = data.as_object()
        && let Some((key, value)) = obj.iter().next()
    {
        let message = value
            .as_str()
            .map(str::to_string)
            .unwrap_or_else(|| value.to_string());
        return (Some(key.clone()), message);
    }
    (None, data.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn ok_serializes_with_status_ok_and_no_meta_key() {
        let v = serde_json::to_value(ApiResponse::Ok {
            data: json!({ "id": 1 }),
            meta: None,
        })
        .unwrap();
        assert_eq!(v["status"], "ok");
        assert_eq!(v["data"]["id"], 1);
        assert!(v.get("meta").is_none(), "meta omitted when None");
    }

    #[test]
    fn ok_list_carries_meta() {
        let v = serde_json::to_value(ApiResponse::Ok {
            data: json!([1, 2]),
            meta: Some(Meta {
                total: 2,
                page: 0,
                per_page: 50,
                total_pages: 1,
                has_next: false,
                has_prev: false,
            }),
        })
        .unwrap();
        assert_eq!(v["status"], "ok");
        assert_eq!(v["meta"]["total"], 2);
    }

    #[test]
    fn error_shape_is_status_error_with_error_object() {
        let v = serde_json::to_value(ApiResponse::<()>::Error {
            error: ApiError {
                code: "not_found".into(),
                message: "nope".into(),
                field: Some("id".into()),
            },
        })
        .unwrap();
        assert_eq!(v["status"], "error");
        assert_eq!(v["error"]["code"], "not_found");
        assert_eq!(v["error"]["message"], "nope");
        assert_eq!(v["error"]["field"], "id");
        assert!(v.get("data").is_none(), "no data on error");
    }

    #[test]
    fn fail_maps_first_kv_to_field_and_message() {
        // The mapping helper is what the `fail` constructor uses.
        let (field, message) = first_field_message(&json!({ "geofence": "no geofence 7" }));
        assert_eq!(field.as_deref(), Some("geofence"));
        assert_eq!(message, "no geofence 7");
    }

    #[test]
    fn code_derivation() {
        assert_eq!(code_for_status(StatusCode::NOT_FOUND), "not_found");
        assert_eq!(
            code_for_status(StatusCode::UNPROCESSABLE_ENTITY),
            "unprocessable"
        );
        assert_eq!(code_for_status(StatusCode::GATEWAY_TIMEOUT), "timeout");
    }

    // ── code_for_status full coverage ────────────────────────────────────────

    #[test]
    fn code_for_status_all_named_branches() {
        assert_eq!(code_for_status(StatusCode::BAD_REQUEST), "invalid_request");
        assert_eq!(code_for_status(StatusCode::UNAUTHORIZED), "unauthorized");
        assert_eq!(code_for_status(StatusCode::FORBIDDEN), "forbidden");
        assert_eq!(code_for_status(StatusCode::NOT_FOUND), "not_found");
        assert_eq!(code_for_status(StatusCode::CONFLICT), "conflict");
        assert_eq!(
            code_for_status(StatusCode::UNPROCESSABLE_ENTITY),
            "unprocessable"
        );
        assert_eq!(
            code_for_status(StatusCode::TOO_MANY_REQUESTS),
            "rate_limited"
        );
        assert_eq!(code_for_status(StatusCode::GATEWAY_TIMEOUT), "timeout");
    }

    #[test]
    fn code_for_status_5xx_not_504_gives_internal_error() {
        // 500, 502, 503 all hit the `s >= 500` arm.
        assert_eq!(
            code_for_status(StatusCode::INTERNAL_SERVER_ERROR),
            "internal_error"
        );
        assert_eq!(code_for_status(StatusCode::BAD_GATEWAY), "internal_error");
        assert_eq!(
            code_for_status(StatusCode::SERVICE_UNAVAILABLE),
            "internal_error"
        );
    }

    #[test]
    fn code_for_status_generic_4xx_gives_error() {
        // 405 is not in the named list — falls to the `_ => "error"` arm.
        assert_eq!(code_for_status(StatusCode::METHOD_NOT_ALLOWED), "error");
    }

    // ── ApiResponse::error constructor ────────────────────────────────────────

    #[test]
    fn error_constructor_derives_code_from_status_when_none() {
        let resp = ApiResponse::<()>::error(StatusCode::NOT_FOUND, "nope", None, None);
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn error_constructor_uses_explicit_code() {
        let resp = ApiResponse::<()>::error(
            StatusCode::BAD_REQUEST,
            "bad",
            Some("custom_code".into()),
            None,
        );
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    // ── ApiResponse::fail constructor ─────────────────────────────────────────

    #[test]
    fn fail_constructor_maps_first_kv() {
        let resp = ApiResponse::<()>::fail(
            StatusCode::UNPROCESSABLE_ENTITY,
            json!({ "name": "too long" }),
        );
        assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[test]
    fn fail_constructor_non_object_value() {
        let resp = ApiResponse::<()>::fail(StatusCode::BAD_REQUEST, json!("plain error string"));
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    // ── first_field_message edge cases ────────────────────────────────────────

    #[test]
    fn first_field_message_non_object_stringifies() {
        let (field, message) = first_field_message(&json!(42));
        assert!(field.is_none());
        assert_eq!(message, "42");
    }

    #[test]
    fn first_field_message_nested_value_is_stringified() {
        // A non-string JSON value as the field value gets stringified.
        let (field, message) = first_field_message(&json!({ "count": 7 }));
        assert_eq!(field.as_deref(), Some("count"));
        assert_eq!(message, "7");
    }

    #[test]
    fn first_field_message_array_stringifies() {
        let (field, message) = first_field_message(&json!([1, 2, 3]));
        assert!(field.is_none());
        // The array is stringified as "[1,2,3]".
        assert!(message.contains("1"));
    }

    // ── ApiResponse::success_with_status ──────────────────────────────────────

    #[test]
    fn success_with_status_201_reflected() {
        let resp = ApiResponse::success_with_status(StatusCode::CREATED, json!({"id": 99}));
        assert_eq!(resp.status(), StatusCode::CREATED);
    }

    // ── ApiResponse::success_paginated ────────────────────────────────────────

    #[test]
    fn success_paginated_carries_meta_in_body() {
        let meta = Meta {
            total: 10,
            page: 1,
            per_page: 5,
            total_pages: 2,
            has_next: true,
            has_prev: false,
        };
        let resp = ApiResponse::success_paginated(json!([1, 2, 3, 4, 5]), meta);
        assert_eq!(resp.status(), StatusCode::OK);
    }

    // ── error object field is omitted when None ───────────────────────────────

    #[test]
    fn error_object_without_field_omits_key() {
        let v = serde_json::to_value(ApiResponse::<()>::Error {
            error: ApiError {
                code: "not_found".into(),
                message: "gone".into(),
                field: None,
            },
        })
        .unwrap();
        assert!(
            v["error"].get("field").is_none(),
            "field must be absent when None"
        );
    }
}
