use std::collections::HashSet;

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

    Ok(HttpResponse::Ok().json(Response {
        data: Some(json!(result)),
        message: "Success".to_string(),
        status: "ok".to_string(),
        stats: None,
        status_code: 200,
    }))
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

    Ok(HttpResponse::Ok().json(Response {
        data: Some(json!(result)),
        message: "Success".to_string(),
        status: "ok".to_string(),
        stats: None,
        status_code: 200,
    }))
}

#[post("/polygons")]
async fn cell_polygons(payload: web::Json<Vec<String>>) -> Result<HttpResponse, Error> {
    let cell_ids = payload.into_inner();
    let result = s2::get_polygons(cell_ids);

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
    let all = bounds.ids.is_none();
    let ids = if let Some(ids) = bounds.ids {
        ids.into_iter().collect::<HashSet<String>>()
    } else {
        HashSet::new()
    };

    let cell_level = url.into_inner();
    let cells = s2::get_cells(
        cell_level,
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

    Ok(HttpResponse::Ok().json(Response {
        data: Some(json!(cells)),
        message: "Success".to_string(),
        status: "ok".to_string(),
        stats: None,
        status_code: 200,
    }))
}
