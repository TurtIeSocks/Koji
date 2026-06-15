//! v2 S2 cell helpers — `POST /api/v2/s2/{circle-coverage,cell-coverage,polygons,{level}}`
//! (architecture §4.4, §6). Lifted out of the old `geo` grab-bag (where they
//! nested under an `s2` sub-scope) into their own top-level `/s2` namespace.
//! Their output is cell-id arrays / coverage objects / polygons
//! (not negotiable GeoJSON), so they keep the enveloped [`ApiResponse`] shape
//! rather than `respond_geo`; the handler signatures adopt
//! [`ServiceError`](crate::utils::error::ServiceError) for surface uniformity with
//! the rest of the v2 API.
//!
// The S2 ops are pure computations with no failure path today, but carry the
// `Result<HttpResponse, ServiceError>` signature for surface uniformity with the
// rest of the v2 handlers (so a future fallible step is a non-breaking change).
#![allow(clippy::unnecessary_wraps)]

use actix_web::{HttpResponse, post, web};
use koji_core::BoundsArg;
use serde::Deserialize;
use serde_json::json;
use std::collections::HashSet;
use utoipa::ToSchema;

use crate::utils::api_response::ApiResponse;
use crate::utils::error::ServiceError;

/// Request for the S2 circle/cell coverage helpers.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub(crate) struct CoverageArgs {
    lat: f64,
    lon: f64,
    radius: Option<f64>,
    size: Option<u8>,
    level: u8,
}

/// Documentation-only mirror of the `POST /api/v2/s2/{cell_level}` body — the
/// foreign [`koji_core::BoundsArg`] (a flattened [`koji_core::KojiBbox`] plus
/// `last_seen`/`ids`/`tth`) carries no `utoipa` dep, so this `ToSchema` twin
/// describes that wire shape for the generated OpenAPI without forcing utoipa
/// into koji-core. Never constructed; referenced only as the `request_body`.
#[derive(Debug, ToSchema)]
#[allow(dead_code)]
pub(crate) struct S2CellsBody {
    /// Bounding box minimum latitude (flattened from `KojiBbox`).
    min_lat: f64,
    /// Bounding box minimum longitude.
    min_lon: f64,
    /// Bounding box maximum latitude.
    max_lat: f64,
    /// Bounding box maximum longitude.
    max_lon: f64,
    /// Optional "updated within the last N seconds" filter.
    last_seen: Option<u32>,
    /// Optional S2 cell-id allow-list; when omitted, every cell in the bbox.
    ids: Option<Vec<String>>,
    /// Optional spawnpoint confirmed/unconfirmed filter (`All`/`Known`/`Unknown`).
    #[schema(value_type = String)]
    tth: Option<koji_core::SpawnpointTth>,
}

/// `POST /api/v2/s2/circle-coverage` — S2 cells covering a circle.
#[post("/circle-coverage")]
async fn circle_coverage(
    payload: web::Json<CoverageArgs>,
) -> Result<HttpResponse, ServiceError> {
    let CoverageArgs {
        lat,
        lon,
        radius,
        level,
        ..
    } = payload.into_inner();
    let result = koji_core::s2::circle_coverage(lat, lon, radius.unwrap_or(70.), level);
    Ok(ApiResponse::success(json!(result)))
}

/// `POST /api/v2/s2/cell-coverage` — S2 cell ids covering a cell area.
#[post("/cell-coverage")]
async fn cell_coverage(payload: web::Json<CoverageArgs>) -> Result<HttpResponse, ServiceError> {
    let CoverageArgs {
        lat,
        lon,
        level,
        size,
        ..
    } = payload.into_inner();
    let result = koji_core::s2::cell_coverage(lat, lon, size.unwrap_or(15), level);
    let locked = result
        .iter()
        .map(|cell| cell.to_string())
        .collect::<Vec<String>>();
    Ok(ApiResponse::success(json!(locked)))
}

/// `POST /api/v2/s2/polygons` — polygons for the given S2 cell ids.
#[post("/polygons")]
async fn cell_polygons(payload: web::Json<Vec<String>>) -> Result<HttpResponse, ServiceError> {
    let result = koji_core::s2::get_polygons(payload.into_inner());
    Ok(ApiResponse::success(json!(result)))
}

/// `POST /api/v2/s2/{cell_level}` — S2 cells within a bbox at `cell_level`,
/// optionally filtered to a set of ids. Ports v1 `/s2/{cell_level}`.
#[post("/{cell_level}")]
async fn s2_cells(
    payload: web::Json<BoundsArg>,
    url: web::Path<u8>,
) -> Result<HttpResponse, ServiceError> {
    let bounds = payload.into_inner();
    let all = bounds.ids.is_none();
    let ids = bounds
        .ids
        .clone()
        .map(|ids| ids.into_iter().collect::<HashSet<String>>())
        .unwrap_or_default();

    let cells = koji_core::s2::get_cells(
        url.into_inner(),
        bounds.bbox.min_lat,
        bounds.bbox.min_lon,
        bounds.bbox.max_lat,
        bounds.bbox.max_lon,
    );
    let cells = if all {
        cells
    } else {
        cells
            .into_iter()
            .filter(|cell| ids.contains(&cell.id))
            .collect()
    };
    Ok(ApiResponse::success(json!(cells)))
}

/// The `/s2` scope. The named routes register before the `/{cell_level}`
/// catch-all so they match first. Mounted into `/api/v2` by [`crate::start`].
pub(crate) fn scope() -> actix_web::Scope {
    web::scope("/s2")
        .service(circle_coverage)
        .service(cell_coverage)
        .service(cell_polygons)
        .service(s2_cells)
}
