use super::DbPool;

use actix_web::{get, post, web, Error, HttpResponse};

use crate::marker_gen::{build_gyms, build_pokestops, build_spawnpoints};
use crate::models::Body;
use crate::queries::{find_all_gyms, find_all_pokestops, find_all_spawnpoints, find_spawnpoints};

#[get("/config")]
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

#[get("/all_spawnpoints")]
async fn all_spawnpoints(pool: web::Data<DbPool>) -> Result<HttpResponse, Error> {
    let all_spawnpoints = web::block(move || {
        let conn = pool.get()?;
        find_all_spawnpoints(&conn)
    })
    .await?
    .map_err(actix_web::error::ErrorInternalServerError)?;
    let markers = build_spawnpoints(&all_spawnpoints);
    Ok(HttpResponse::Ok().json(markers))
}

#[post("/spawnpoints")]
async fn spawnpoints(
    pool: web::Data<DbPool>,
    payload: web::Json<Body>,
) -> Result<HttpResponse, Error> {
    let spawnpoints = web::block(move || {
        let conn = pool.get()?;
        find_spawnpoints(&conn, &payload)
    })
    .await?
    .map_err(actix_web::error::ErrorInternalServerError)?;
    let markers = build_spawnpoints(&spawnpoints);
    Ok(HttpResponse::Ok().json(markers))
}

#[get("/gyms")]
async fn gyms(pool: web::Data<DbPool>) -> Result<HttpResponse, Error> {
    let gyms = web::block(move || {
        let conn = pool.get()?;
        find_all_gyms(&conn)
    })
    .await?
    .map_err(actix_web::error::ErrorInternalServerError)?;
    let markers = build_gyms(&gyms);
    Ok(HttpResponse::Ok().json(markers))
}

#[get("/pokestops")]
async fn pokestops(pool: web::Data<DbPool>) -> Result<HttpResponse, Error> {
    let pokestops = web::block(move || {
        let conn = pool.get()?;
        find_all_pokestops(&conn)
    })
    .await?
    .map_err(actix_web::error::ErrorInternalServerError)?;
    let markers = build_pokestops(&pokestops);
    Ok(HttpResponse::Ok().json(markers))
}
