//! Shared helpers for the geofence/route CRUD handlers — the create tail,
//! PATCH overlay-merge, and DELETE-404 shape were previously near-verbatim
//! copies in both files.

use actix_web::{HttpResponse, http::StatusCode};

use crate::utils::api_response::ApiResponse;
use crate::utils::error::ServiceError;

/// Shallow-overlay `patch` onto `base` (top-level keys only — the PATCH body's
/// present fields replace the stored row's).
pub(crate) fn merge_patch(base: &mut serde_json::Value, patch: &serde_json::Value) {
    if let (Some(base), Some(patch)) = (base.as_object_mut(), patch.as_object()) {
        for (k, v) in patch {
            base.insert(k.clone(), v.clone());
        }
    }
}

/// The numeric `id` of a freshly upserted record. A missing/non-numeric id is
/// a 500 with context — the old inline `unwrap_or(0)` silently emitted id 0
/// into events and the Location header.
pub(crate) fn record_id(record: &serde_json::Value) -> Result<u64, ServiceError> {
    record
        .get("id")
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| ServiceError::internal("upsert returned a record without a numeric id"))
}

/// The `201 Created` response: `Location: {prefix}/{id}` + enveloped record.
pub(crate) fn created_response(prefix: &str, id: u64, record: serde_json::Value) -> HttpResponse {
    HttpResponse::build(StatusCode::CREATED)
        .insert_header(("Location", format!("{prefix}/{id}")))
        .json(ApiResponse::Ok {
            data: record,
            meta: None,
        })
}

/// Map a delete's `rows_affected == 0` to the standard 404.
pub(crate) fn delete_or_404(rows_affected: u64, entity: &'static str) -> Result<(), ServiceError> {
    if rows_affected == 0 {
        return Err(ServiceError::NotFound {
            field: entity,
            message: "does not exist".to_string(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn merge_patch_overlays_top_level_keys() {
        let mut base = json!({ "a": 1, "b": 2 });
        merge_patch(&mut base, &json!({ "b": 9, "c": 3 }));
        assert_eq!(base, json!({ "a": 1, "b": 9, "c": 3 }));
    }

    #[test]
    fn record_id_missing_is_internal_error_not_zero() {
        assert!(record_id(&json!({ "name": "x" })).is_err());
        assert_eq!(record_id(&json!({ "id": 7 })).unwrap(), 7);
    }

    #[test]
    fn delete_or_404_maps_zero_rows() {
        assert!(delete_or_404(0, "route").is_err());
        assert!(delete_or_404(1, "route").is_ok());
    }
}
