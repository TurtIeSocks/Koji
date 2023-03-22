use super::*;

use algorithms::s2;

use model::api::args::{BoundsArg, Response};
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

#[post("/cell-coverage")]
async fn cell_coverage(payload: web::Json<CoverageArgs>) -> Result<HttpResponse, Error> {
    let CoverageArgs {
        lat,
        lon,
        radius,
        level,
        size,
    } = payload.into_inner();

    let result = if let Some(radius) = radius {
        Some(s2::circle_coverage(lat, lon, radius, level))
    } else if let Some(size) = size {
        Some(s2::cell_coverage(lat, lon, size, level))
    } else {
        None
    };

    Ok(HttpResponse::Ok().json(Response {
        data: Some(json!(result)),
        message: "Success".to_string(),
        status: "ok".to_string(),
        stats: None,
        status_code: 200,
    }))
}

#[post("/{cell_level}")]
async fn s2_cells(
    payload: web::Json<BoundsArg>,
    url: actix_web::web::Path<u8>,
) -> Result<HttpResponse, Error> {
    let bounds = payload.into_inner();
    let cell_level = url.into_inner();
    let cells = s2::get_cells(
        cell_level,
        bounds.min_lat,
        bounds.min_lon,
        bounds.max_lat,
        bounds.max_lon,
    );

    Ok(HttpResponse::Ok().json(Response {
        data: Some(json!(cells)),
        message: "Success".to_string(),
        status: "ok".to_string(),
        stats: None,
        status_code: 200,
    }))
}
