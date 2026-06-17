//! v2 app-config bootstrap — `GET /api/v2/config`.
//!
//! Ports the legacy `/config` (`private::misc::config`) blob onto the v2 surface:
//! the frontend's boot payload — map start center, tile server, plugin lists, and
//! the session login flag — now wrapped in the [`ApiResponse`] envelope. Distinct
//! from `/api/v2/auth/me` (which reports *only* the request's auth standing); this
//! is the broader app-bootstrap config.
//!
//! Reuses the existing [`ConfigResponse`] struct and the same data sources (env
//! vars + session + the `algorithms` plugin registries), so the shape is identical
//! to v1 modulo the envelope.

use actix_session::Session;
use koji_core::Precision;
use actix_web::{HttpResponse, get};

use algorithms::{bootstrap, clustering, routing};

use crate::utils::api_response::ApiResponse;
use crate::utils::error::ServiceError;
use crate::utils::response::ConfigResponse;

/// `GET /api/v2/config` — the app bootstrap blob. Ports v1 `/config`: reads
/// `START_LAT`/`START_LON`/`TILE_SERVER`/`DANGEROUS` from the env, the
/// `logged_in` flag from the session, and the three plugin lists from the
/// `algorithms` registries, returned via [`ApiResponse`].
#[utoipa::path(
    get,
    path = "/api/v2/config",
    tag = "config",
    responses((status = 200, description = "App bootstrap blob", body = ConfigResponse)),
)]
// `ServiceError` is intentionally large (crate-wide allow); this handler never
// errors today, but keeps the `Result<_, ServiceError>` signature for surface
// consistency with the other v2 handlers.
#[allow(clippy::result_large_err)]
#[get("/config")]
pub(crate) async fn config(session: Session) -> Result<HttpResponse, ServiceError> {
    let start_lat: Precision = std::env::var("START_LAT")
        .unwrap_or_else(|_| "0.0".to_string())
        .parse()
        .unwrap_or(0.0);
    let start_lon: Precision = std::env::var("START_LON")
        .unwrap_or_else(|_| "0.0".to_string())
        .parse()
        .unwrap_or(0.0);

    let tile_server = std::env::var("TILE_SERVER").unwrap_or_default();
    let route_plugins = routing::routing_plugins();
    let clustering_plugins = clustering::clustering_plugins();
    let bootstrap_plugins = bootstrap::bootstrap_plugins();

    Ok(ApiResponse::success(ConfigResponse {
        start_lat,
        start_lon,
        tile_server,
        logged_in: session
            .get::<bool>("logged_in")
            .ok()
            .flatten()
            .unwrap_or(false),
        dangerous: std::env::var("DANGEROUS").is_ok(),
        route_plugins,
        clustering_plugins,
        bootstrap_plugins,
    }))
}
