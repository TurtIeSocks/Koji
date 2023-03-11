use super::*;

use algorithms::s2::get_cells;

use model::api::args::{BoundsArg, Response};
use serde_json::json;

#[post("/{cell_level}")]
async fn s2_cells(
    payload: web::Json<BoundsArg>,
    url: actix_web::web::Path<u8>,
) -> Result<HttpResponse, Error> {
    let bounds = payload.into_inner();
    let cell = url.into_inner();
    let feature = get_cells(
        cell,
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
