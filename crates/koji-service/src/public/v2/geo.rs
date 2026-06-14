//! v2 geometry utilities — `POST /api/v2/geo/{convert,simplify,merge-points}`
//! and the S2 helpers under `/api/v2/geo/s2/*` (architecture §6). These port the
//! v1 `/convert` + `/s2` handlers verbatim onto the v2 surface: the geometry
//! transforms honor `?rt=`/`?format=` via [`utils::response::send`]; the S2
//! coverage helpers return plain JSON in the [`ApiResponse`] envelope.

use actix_web::{Error, HttpResponse, post, web};
use geojson::FeatureCollection;
use koji_core::{BoundsArg, FeatureHelpers, GeometryHelpers, TrimPrecision};
use model::api::args::{Args, ArgsUnwrapped};
use serde::Deserialize;
use serde_json::json;
use std::collections::HashSet;

use crate::utils::{self, api_response::ApiResponse};

/// `POST /api/v2/geo/convert` — convert/normalize a geometry to the requested
/// return type, optionally simplifying. Ports v1 `/convert/data`.
#[post("/convert")]
async fn convert(payload: web::Json<Args>) -> Result<HttpResponse, Error> {
    let ArgsUnwrapped {
        area,
        benchmark_mode,
        return_type,
        instance,
        simplify: arg_simplify,
        ..
    } = payload.into_inner().init(Some("convert_data"));

    let area = if arg_simplify { area.simplify() } else { area }
        .into_iter()
        .map(|feat| feat.remove_internal_props())
        .collect::<FeatureCollection>()
        .trim_precision(6);

    let coll = koji_core::KojiGeometryCollection::try_from(area)
        .map_err(actix_web::error::ErrorInternalServerError)?;

    Ok(utils::response::send(
        coll,
        return_type,
        None,
        benchmark_mode,
        Some(instance),
    ))
}

/// `POST /api/v2/geo/simplify` — simplify the supplied geometry. Ports v1
/// `/convert/simplify`.
#[post("/simplify")]
async fn simplify(payload: web::Json<Args>) -> Result<HttpResponse, Error> {
    let ArgsUnwrapped {
        area, return_type, ..
    } = payload.into_inner().init(Some("simplify"));

    let coll = koji_core::KojiGeometryCollection::try_from(area.simplify())
        .map_err(actix_web::error::ErrorInternalServerError)?;

    Ok(utils::response::send(coll, return_type, None, false, None))
}

/// `POST /api/v2/geo/merge-points` — merge point features into one MultiPoint.
/// Ports v1 `/convert/merge-points`.
#[post("/merge-points")]
async fn merge_points(payload: web::Json<Args>) -> Result<HttpResponse, Error> {
    let ArgsUnwrapped {
        area, return_type, ..
    } = payload.into_inner().init(Some("simplify"));

    // Koji-native: normalize the inbound geometry, then collapse its point items
    // into one MultiPoint via the re-homed `KojiGeometryCollection::merge_points`.
    // No `To*` matrix.
    let coll = koji_core::KojiGeometryCollection::try_from(area)
        .map_err(actix_web::error::ErrorInternalServerError)?
        .merge_points();

    Ok(utils::response::send(coll, return_type, None, false, None))
}

/// Request for the S2 circle/cell coverage helpers.
#[derive(Debug, Clone, Deserialize)]
struct CoverageArgs {
    lat: f64,
    lon: f64,
    radius: Option<f64>,
    size: Option<u8>,
    level: u8,
}

/// `POST /api/v2/geo/s2/circle-coverage` — S2 cells covering a circle.
#[post("/circle-coverage")]
async fn circle_coverage(payload: web::Json<CoverageArgs>) -> Result<HttpResponse, Error> {
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

/// `POST /api/v2/geo/s2/cell-coverage` — S2 cell ids covering a cell area.
#[post("/cell-coverage")]
async fn cell_coverage(payload: web::Json<CoverageArgs>) -> Result<HttpResponse, Error> {
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

/// `POST /api/v2/geo/s2/polygons` — polygons for the given S2 cell ids.
#[post("/polygons")]
async fn cell_polygons(payload: web::Json<Vec<String>>) -> Result<HttpResponse, Error> {
    let result = koji_core::s2::get_polygons(payload.into_inner());
    Ok(ApiResponse::success(json!(result)))
}

/// `POST /api/v2/geo/s2/{cell_level}` — S2 cells within a bbox at `cell_level`,
/// optionally filtered to a set of ids. Ports v1 `/s2/{cell_level}`.
#[post("/{cell_level}")]
async fn s2_cells(
    payload: web::Json<BoundsArg>,
    url: web::Path<u8>,
) -> Result<HttpResponse, Error> {
    let bounds = payload.into_inner();
    let all = bounds.ids.is_none();
    let ids = bounds
        .ids
        .clone()
        .map(|ids| ids.into_iter().collect::<HashSet<String>>())
        .unwrap_or_default();

    let cells = koji_core::s2::get_cells(
        url.into_inner(),
        bounds.min_lat,
        bounds.min_lon,
        bounds.max_lat,
        bounds.max_lon,
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

/// The `/geo` scope. The S2 helpers nest under `/geo/s2`; the named s2 routes
/// register before the `/{cell_level}` catch-all so they match first.
pub fn scope() -> actix_web::Scope {
    web::scope("/geo")
        .service(convert)
        .service(simplify)
        .service(merge_points)
        .service(
            web::scope("/s2")
                .service(circle_coverage)
                .service(cell_coverage)
                .service(cell_polygons)
                .service(s2_cells),
        )
}
