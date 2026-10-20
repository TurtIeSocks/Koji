//! `ServiceError` — the koji-service error type that maps cleanly to the v2
//! response envelope. Handlers return `Result<HttpResponse, ServiceError>`;
//! actix renders the error via `ResponseError` into
//! `{ "status": "error", "error": { code, message, field? } }` at the right HTTP
//! status. Replaces the scattered
//! `.map_err(actix_web::error::ErrorInternalServerError)` calls.
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
    Conflict {
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
    /// A call to an upstream service (Dragonite) failed: 502, with the upstream
    /// detail in the message.
    #[error("{message}")]
    Upstream {
        field: &'static str,
        message: String,
    },
    #[error(transparent)]
    Db(#[from] DbErr),
    #[error(transparent)]
    Model(ModelError),
    /// An internal failure carrying a context string. Logged; the client sees a
    /// generic 500 (detail not leaked). Funnels plumbing errors (enqueue,
    /// serialize, non-NotFound await) that have no meaningful client-facing code.
    #[error("{0}")]
    Internal(String),
}

/// An ambiguous geofence name is the caller's to resolve, not an internal
/// failure: 409 naming the modes, whichever handler's `?` it surfaces in.
impl From<ModelError> for ServiceError {
    fn from(e: ModelError) -> Self {
        match e {
            ModelError::AmbiguousGeofence { name, modes } => {
                ServiceError::ambiguous_geofence(&name, &modes, false)
            }
            e => ServiceError::Model(e),
        }
    }
}

impl ServiceError {
    /// The 409 for a geofence name held by several modes. `accepts_mode` is
    /// whether the endpoint reads `?mode=`; if not, the way out is the id.
    fn ambiguous_geofence(name: &str, modes: &[String], accepts_mode: bool) -> Self {
        let advice = if accepts_mode {
            "pass ?mode= or use its id to pick one"
        } else {
            "use its id to pick one"
        };
        ServiceError::Conflict {
            field: "geofence",
            message: format!(
                "geofence `{name}` exists in modes: {}; {advice}",
                modes.join(", ")
            ),
        }
    }

    /// Build an `Internal` error from any displayable error.
    pub(crate) fn internal(e: impl std::fmt::Display) -> Self {
        ServiceError::Internal(e.to_string())
    }

    /// `map_err` adapter for `get_one`-style lookups: a genuine DB failure
    /// (connection loss, timeout) stays a 500, everything else (row missing)
    /// becomes the given 404. A blanket `|_| NotFound` here would make an
    /// outage read as "record deleted" to clients and as 404s to monitoring.
    pub(crate) fn not_found_or_db(
        field: &'static str,
        message: String,
    ) -> impl FnOnce(ModelError) -> Self {
        Self::lookup_error(field, message, false)
    }

    /// [`ServiceError::not_found_or_db`] for an endpoint that reads `?mode=`, so
    /// the 409 for a shared geofence name can point at it.
    pub(crate) fn not_found_or_db_pick_mode(
        field: &'static str,
        message: String,
    ) -> impl FnOnce(ModelError) -> Self {
        Self::lookup_error(field, message, true)
    }

    fn lookup_error(
        field: &'static str,
        message: String,
        accepts_mode: bool,
    ) -> impl FnOnce(ModelError) -> Self {
        move |e| match e {
            ModelError::Database(_) => ServiceError::Model(e),
            ModelError::AmbiguousGeofence { name, modes } => {
                ServiceError::ambiguous_geofence(&name, &modes, accepts_mode)
            }
            _ => ServiceError::NotFound { field, message },
        }
    }

    /// Pure mapping to `(status, error-object)`. Unit-tested directly; the
    /// `ResponseError` impl is a thin wrapper over this. Internal failures
    /// (`Db`/`Model`) are logged and surface a generic message — never leaking
    /// their detail to the client.
    pub(crate) fn to_api_error(&self) -> (StatusCode, ApiError) {
        let (status, field, message) = match self {
            ServiceError::NotFound { field, message } => (
                StatusCode::NOT_FOUND,
                Some((*field).to_string()),
                message.clone(),
            ),
            ServiceError::Conflict { field, message } => (
                StatusCode::CONFLICT,
                Some((*field).to_string()),
                message.clone(),
            ),
            ServiceError::Invalid { field, message } => {
                (StatusCode::BAD_REQUEST, field.clone(), message.clone())
            }
            ServiceError::Unprocessable { field, message } => (
                StatusCode::UNPROCESSABLE_ENTITY,
                field.clone(),
                message.clone(),
            ),
            ServiceError::Upstream { field, message } => (
                StatusCode::BAD_GATEWAY,
                Some((*field).to_string()),
                message.clone(),
            ),
            ServiceError::Db(e) => {
                log::error!("service db error: {e}");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    None,
                    "internal error".to_string(),
                )
            }
            ServiceError::Model(e) => {
                log::error!("service model error: {e}");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    None,
                    "internal error".to_string(),
                )
            }
            ServiceError::Internal(detail) => {
                log::error!("service internal error: {detail}");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    None,
                    "internal error".to_string(),
                )
            }
        };
        (
            status,
            ApiError {
                code: code_for_status(status),
                message,
                field,
            },
        )
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
    fn upstream_maps_to_502_with_its_detail() {
        let (status, err) = ServiceError::Upstream {
            field: "dragonite",
            message: "Dragonite answered 401: bad token".into(),
        }
        .to_api_error();
        assert_eq!(status, StatusCode::BAD_GATEWAY);
        assert_eq!(err.field.as_deref(), Some("dragonite"));
        assert_eq!(err.message, "Dragonite answered 401: bad token");
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
    fn db_error_is_500_generic_and_does_not_leak() {
        let (status, err) =
            ServiceError::Db(DbErr::Custom("secret internal detail".into())).to_api_error();
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(err.code, "internal_error");
        assert_eq!(err.message, "internal error");
        assert!(
            !err.message.contains("secret"),
            "internal detail must not leak"
        );
    }

    #[test]
    fn internal_is_500_generic_and_does_not_leak() {
        let (status, err) = ServiceError::internal("secret plumbing detail").to_api_error();
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(err.code, "internal_error");
        assert_eq!(err.message, "internal error");
        assert!(!err.message.contains("secret"));
    }

    #[test]
    fn not_found_or_db_keeps_db_failures_as_500() {
        let map = ServiceError::not_found_or_db("geofence", "no geofence 7".into());
        let e = map(ModelError::Database(DbErr::Custom("conn lost".into())));
        let (status, _) = e.to_api_error();
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn not_found_or_db_maps_missing_row_to_404() {
        let map = ServiceError::not_found_or_db("geofence", "no geofence 7".into());
        let e = map(ModelError::Geofence("Does not exist".into()));
        let (status, err) = e.to_api_error();
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(err.field.as_deref(), Some("geofence"));
    }

    #[test]
    fn conflict_maps_to_409() {
        let (status, err) = ServiceError::Conflict {
            field: "geofence",
            message: "taken".into(),
        }
        .to_api_error();
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(err.code, "conflict");
        assert_eq!(err.field.as_deref(), Some("geofence"));
    }

    #[test]
    fn not_found_or_db_maps_an_ambiguous_name_to_409_naming_the_modes() {
        let map = ServiceError::not_found_or_db("geofence", "no geofence Boston".into());
        let e = map(ModelError::AmbiguousGeofence {
            name: "Boston".into(),
            modes: vec!["pokemon".into(), "unset".into()],
        });
        let (status, err) = e.to_api_error();
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(err.field.as_deref(), Some("geofence"));
        assert!(err.message.contains("Boston"), "{}", err.message);
        assert!(err.message.contains("pokemon, unset"), "{}", err.message);
        // This adapter's endpoints take no `?mode=`; the advice must be one that works.
        assert!(err.message.contains("id"), "{}", err.message);
        assert!(!err.message.contains("?mode="), "{}", err.message);
    }

    #[test]
    fn not_found_or_db_pick_mode_points_at_the_mode_param() {
        let map = ServiceError::not_found_or_db_pick_mode("geofence", "no geofence Boston".into());
        let e = map(ModelError::AmbiguousGeofence {
            name: "Boston".into(),
            modes: vec!["pokemon".into(), "unset".into()],
        });
        let (status, err) = e.to_api_error();
        assert_eq!(status, StatusCode::CONFLICT);
        assert!(err.message.contains("pokemon, unset"), "{}", err.message);
        assert!(err.message.contains("?mode="), "{}", err.message);
    }

    #[test]
    fn an_ambiguous_geofence_through_question_mark_is_409_not_500() {
        fn lookup() -> Result<(), ServiceError> {
            Err(ModelError::AmbiguousGeofence {
                name: "Boston".into(),
                modes: vec!["pokemon".into(), "unset".into()],
            })?;
            Ok(())
        }
        let (status, err) = lookup().unwrap_err().to_api_error();
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(err.field.as_deref(), Some("geofence"));
        assert!(err.message.contains("Boston"), "{}", err.message);
        assert!(err.message.contains("pokemon, unset"), "{}", err.message);
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
