//! `ServiceError` — the koji-service error type that maps cleanly to the v2
//! response envelope. Handlers return `Result<HttpResponse, ServiceError>`;
//! actix renders the error via `ResponseError` into
//! `{ "status": "error", "error": { code, message, field? } }` at the right HTTP
//! status. Replaces the scattered
//! `.map_err(actix_web::error::ErrorInternalServerError)` calls.
//!
//! Phase 0 lands this type ahead of its consumers: the v2 handlers that return
//! `Result<HttpResponse, ServiceError>` (and the db-layer `NotFound` signal that
//! maps to a 404) are wired in P1/P2. Until then `ServiceError` is constructed
//! only by the unit tests below, so the non-test build sees it as dead —
//! silenced crate-wide for this module rather than item-by-item.
#![allow(dead_code)]
// `ServiceError` carries sea-orm's `DbErr` and koji-db's `ModelError` by value
// (≥200 bytes) so handlers get ergonomic `?` and exhaustive `match` on the
// typed variants — the deliberate design here. Boxing those to satisfy
// `result_large_err` would erase that surface for marginal stack savings on the
// cold error path, so the lint is allowed for this module.
#![allow(clippy::result_large_err)]

use actix_web::{HttpResponse, ResponseError, http::StatusCode};
use koji_db::ModelError;
use migration::DbErr;
use thiserror::Error;

use crate::utils::api_response::{ApiError, ApiResponse, code_for_status};

#[derive(Debug, Error)]
pub(crate) enum ServiceError {
    #[error("{message}")]
    NotFound {
        field: &'static str,
        message: String,
    },
    #[error("{message}")]
    Invalid {
        field: Option<String>,
        message: String,
    },
    #[error("{message}")]
    Unprocessable {
        field: Option<String>,
        message: String,
    },
    #[error("{0}")]
    Conflict(String),
    #[error(transparent)]
    Db(#[from] DbErr),
    #[error(transparent)]
    Model(#[from] ModelError),
}

impl ServiceError {
    /// Pure mapping to `(status, error-object)`. Unit-tested directly; the
    /// `ResponseError` impl is a thin wrapper over this. Internal failures
    /// (`Db`/`Model`) are logged and surface a generic message — never leaking
    /// their detail to the client.
    pub(crate) fn to_api_error(&self) -> (StatusCode, ApiError) {
        let (status, field, message) = match self {
            ServiceError::NotFound { field, message } => {
                (StatusCode::NOT_FOUND, Some((*field).to_string()), message.clone())
            }
            ServiceError::Invalid { field, message } => {
                (StatusCode::BAD_REQUEST, field.clone(), message.clone())
            }
            ServiceError::Unprocessable { field, message } => {
                (StatusCode::UNPROCESSABLE_ENTITY, field.clone(), message.clone())
            }
            ServiceError::Conflict(message) => (StatusCode::CONFLICT, None, message.clone()),
            ServiceError::Db(e) => {
                log::error!("service db error: {e}");
                (StatusCode::INTERNAL_SERVER_ERROR, None, "internal error".to_string())
            }
            ServiceError::Model(e) => {
                log::error!("service model error: {e}");
                (StatusCode::INTERNAL_SERVER_ERROR, None, "internal error".to_string())
            }
        };
        (status, ApiError { code: code_for_status(status), message, field })
    }
}

impl ResponseError for ServiceError {
    fn status_code(&self) -> StatusCode {
        self.to_api_error().0
    }
    fn error_response(&self) -> HttpResponse {
        let (status, error) = self.to_api_error();
        HttpResponse::build(status).json(ApiResponse::<()>::Error { error })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn not_found_maps_to_404_with_field() {
        let (status, err) = ServiceError::NotFound {
            field: "geofence",
            message: "no geofence 7".into(),
        }
        .to_api_error();
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(err.code, "not_found");
        assert_eq!(err.field.as_deref(), Some("geofence"));
        assert_eq!(err.message, "no geofence 7");
    }

    #[test]
    fn invalid_maps_to_400() {
        let (status, err) = ServiceError::Invalid {
            field: Some("radius".into()),
            message: "radius must be positive".into(),
        }
        .to_api_error();
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(err.code, "invalid_request");
        assert_eq!(err.field.as_deref(), Some("radius"));
    }

    #[test]
    fn unprocessable_maps_to_422() {
        let (status, err) = ServiceError::Unprocessable {
            field: Some("dragonite_area_id".into()),
            message: "geofence is not linked".into(),
        }
        .to_api_error();
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(err.code, "unprocessable");
        assert_eq!(err.field.as_deref(), Some("dragonite_area_id"));
    }

    #[test]
    fn conflict_maps_to_409() {
        let (status, err) = ServiceError::Conflict("already canceled".into()).to_api_error();
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(err.code, "conflict");
        assert!(err.field.is_none());
    }

    #[test]
    fn db_error_is_500_generic_and_does_not_leak() {
        let (status, err) = ServiceError::Db(DbErr::Custom("secret internal detail".into()))
            .to_api_error();
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(err.code, "internal_error");
        assert_eq!(err.message, "internal error");
        assert!(!err.message.contains("secret"), "internal detail must not leak");
    }

    #[test]
    fn from_dberr_via_question_mark() {
        fn boom() -> Result<(), ServiceError> {
            Err(DbErr::Custom("x".into()))?;
            Ok(())
        }
        assert!(matches!(boom(), Err(ServiceError::Db(_))));
    }
}
