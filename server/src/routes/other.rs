use super::*;
use crate::models::{api::RouteGeneration, scanner::InstanceData};
use crate::queries::instance::query_instance_route;
use crate::utils::bootstrapping::generate_circles;

#[get("/api/config")]
async fn config() -> Result<HttpResponse, Error> {
    let start_lat: f64 = std::env::var("START_LAT")
        .unwrap_or("0.0".to_string())
        .parse()
        .unwrap();
    let start_lon: f64 = std::env::var("START_LON")
        .unwrap_or("0.0".to_string())
        .parse()
        .unwrap();
    let tile_server = std::env::var("TILE_SERVER").unwrap_or("".to_string());
    let return_value = (start_lat, start_lon, tile_server);
    Ok(HttpResponse::Ok().json(return_value))
}

#[post("/api/bootstrap")]
async fn bootstrap(
    pool: web::Data<DbPool>,
    payload: web::Json<RouteGeneration>,
) -> Result<HttpResponse, Error> {
    let bs_name = payload.instance.clone();
    if bs_name.len() == 0 {
        return Ok(HttpResponse::Ok().json(""));
    }
    let instance = web::block(move || {
        let conn = pool.get()?;

        query_instance_route(&conn, &bs_name)
    })
    .await?
    .map_err(actix_web::error::ErrorInternalServerError)?;

    let data: InstanceData =
        serde_json::from_str(instance.data.as_str()).expect("JSON was not well-formatted");

    let circles = generate_circles(data.area[0].clone(), payload.radius);
    Ok(HttpResponse::Ok().json(circles))
}
