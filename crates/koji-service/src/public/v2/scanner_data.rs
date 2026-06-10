//! v2 scanner-data fetch — `GET /api/v2/scanner-data/{category}` (architecture
//! §6). Returns the scanner points of a category (`gym|pokestop|spawnpoint|
//! station|fort`) within a geofence, reusing the same koji-scanner query path as
//! the v1 `/internal/data/area` handler.
//!
//! **Area input (design choice):** the area is supplied as `?instance=<name>`
//! (a Koji geofence name), resolved via [`utils::create_or_find_collection`] —
//! the simplest read-only GET shape. (The v1 handler took a POST body with an
//! `Args` area; a GET can't, so v2 keys off the stored geofence by name. Ad-hoc
//! bbox queries remain available on the v1 `/internal/data/bound` endpoint.)

use actix_web::{Error, HttpResponse, get, http::StatusCode, web};
use geojson::FeatureCollection;
use koji_core::SpawnpointTth;
use koji_db::KojiDb;
use serde::Deserialize;
use serde_json::json;

use crate::utils::{self, api_response::ApiResponse};

/// Query for `GET /api/v2/scanner-data/{category}`.
#[derive(Debug, Deserialize)]
struct ScannerDataQuery {
    /// Geofence name whose area bounds the fetch (required).
    instance: Option<String>,
    /// Only points updated within the last N seconds (`0` = no filter).
    last_seen: Option<u32>,
}

/// `GET /api/v2/scanner-data/{category}` — the category's scanner points within
/// the `?instance=` geofence, wrapped in the v2 [`ApiResponse`] envelope.
#[get("/scanner-data/{category}")]
async fn scanner_data(
    conn: web::Data<KojiDb>,
    category: web::Path<String>,
    query: web::Query<ScannerDataQuery>,
) -> Result<HttpResponse, Error> {
    let category = category.into_inner();
    let query = query.into_inner();
    let last_seen = query.last_seen.unwrap_or(0);
    let instance = query.instance.unwrap_or_default();
    if instance.is_empty() {
        return Ok(ApiResponse::fail(
            StatusCode::BAD_REQUEST,
            json!({ "instance": "an `instance` (geofence name) query param is required" }),
        ));
    }

    let area = utils::create_or_find_collection(
        &instance,
        &conn,
        FeatureCollection::default(),
        &None,
        &vec![],
    )
    .await
    .map_err(actix_web::error::ErrorInternalServerError)?;

    let points = utils::points_from_area(&area, &category, &conn, last_seen, SpawnpointTth::All)
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;

    Ok(ApiResponse::success(points))
}

/// The `web::Scope` wiring the scanner-data handler, mounted into `/api/v2`.
pub fn scope() -> actix_web::Scope {
    web::scope("").service(scanner_data)
}
