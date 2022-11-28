use super::*;
use crate::models::api::ConfigResponse;

#[get("/config")]
async fn config(scanner_type: web::Data<String>) -> Result<HttpResponse, Error> {
    let scanner_type = scanner_type.as_ref().to_string();
    let start_lat: f64 = std::env::var("START_LAT")
        .unwrap_or("0.0".to_string())
        .parse()
        .unwrap();
    let start_lon: f64 = std::env::var("START_LON")
        .unwrap_or("0.0".to_string())
        .parse()
        .unwrap();
    let tile_server = std::env::var("TILE_SERVER").unwrap_or("".to_string());
    Ok(HttpResponse::Ok().json(ConfigResponse {
        start_lat,
        start_lon,
        tile_server,
        scanner_type,
    }))
}
