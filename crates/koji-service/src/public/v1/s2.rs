use std::collections::HashSet;

use crate::utils::response::ok_response;

use super::*;

use koji_core::s2;

use koji_core::BoundsArg;
use serde::Deserialize;
use serde_json::json;

#[derive(Debug, Clone, Deserialize)]
struct CoverageArgs {
    lat: f64,
    lon: f64,
    radius: Option<f64>,
    size: Option<u8>,
    level: u8,
}

#[post("/circle-coverage")]
async fn circle_coverage(payload: web::Json<CoverageArgs>) -> Result<HttpResponse, Error> {
    let CoverageArgs {
        lat,
        lon,
        radius,
        level,
        ..
    } = payload.into_inner();

    let result = s2::circle_coverage(lat, lon, radius.unwrap_or(70.), level);

    Ok(ok_response(json!(result)))
}

#[post("/cell-coverage")]
async fn cell_coverage(payload: web::Json<CoverageArgs>) -> Result<HttpResponse, Error> {
    let CoverageArgs {
        lat,
        lon,
        level,
        size,
        ..
    } = payload.into_inner();

    let result = s2::cell_coverage(lat, lon, size.unwrap_or(15), level);
    let locked = result
        .iter()
        .map(|cell| cell.to_string())
        .collect::<Vec<String>>();

    Ok(ok_response(json!(locked)))
}

#[post("/polygons")]
async fn cell_polygons(payload: web::Json<Vec<String>>) -> Result<HttpResponse, Error> {
    let cell_ids = payload.into_inner();
    let result = s2::get_polygons(cell_ids);

    Ok(ok_response(json!(result)))
}

#[post("/{cell_level}")]
async fn s2_cells(
    payload: web::Json<BoundsArg>,
    url: actix_web::web::Path<u8>,
) -> Result<HttpResponse, Error> {
    let bounds = payload.into_inner();
    let all = bounds.ids.is_none();
    let ids = if let Some(ids) = bounds.ids {
        ids.into_iter().collect::<HashSet<String>>()
    } else {
        HashSet::new()
    };

    let cell_level = url.into_inner();
    let cells = s2::get_cells(
        cell_level,
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

    Ok(ok_response(json!(cells)))
}
