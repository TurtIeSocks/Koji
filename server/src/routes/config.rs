use super::*;

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
