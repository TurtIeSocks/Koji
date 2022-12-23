use super::*;
use actix_session::Session;
use models::api::{Auth, ConfigResponse};

#[get("/config")]
async fn config(scanner_type: web::Data<String>, session: Session) -> Result<HttpResponse, Error> {
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
        logged_in: if let Ok(logged_in) = session.get::<bool>("logged_in") {
            logged_in.unwrap_or(false)
        } else {
            false
        },
    }))
}

#[post("/login")]
async fn login(payload: web::Json<Auth>, session: Session) -> Result<HttpResponse, Error> {
    if payload.password == std::env::var("KOJI_SECRET").unwrap_or("".to_string()) {
        return match session.insert("logged_in", true) {
            Ok(_) => Ok(HttpResponse::Ok().finish()),
            Err(err) => {
                println!("[API] Error logging in: {:?}", err);
                Ok(HttpResponse::Unauthorized().finish())
            }
        };
    }
    Ok(HttpResponse::Unauthorized().finish())
}
