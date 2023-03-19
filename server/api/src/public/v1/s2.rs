use super::*;

use algorithms::s2::{circle_coverage, get_cells};

use model::api::args::{BoundsArg, Response};
use serde::Deserialize;
use serde_json::json;

#[derive(Debug, Clone, Deserialize)]
struct CircleArgs {
    lat: f64,
    lon: f64,
    radius: f64,
    level: u8,
}

#[post("/circle-coverage")]
async fn circle_intersection(payload: web::Json<CircleArgs>) -> Result<HttpResponse, Error> {
    let CircleArgs {
        lat,
        lon,
        radius,
        level,
    } = payload.into_inner();

    let result = circle_coverage(lat, lon, radius, level);
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
    let feature = get_cells(
        cell_level,
        bounds.min_lat,
        bounds.min_lon,
        bounds.max_lat,
        bounds.max_lon,
    );

    Ok(HttpResponse::Ok().json(Response {
        data: Some(json!(feature)),
        message: "Success".to_string(),
        status: "ok".to_string(),
        stats: None,
        status_code: 200,
    }))
}
