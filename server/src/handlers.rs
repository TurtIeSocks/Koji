use super::DbPool;

use crate::marker_gen::{build_gyms, build_pokestops, build_spawnpoints};
use crate::models::{InstanceData, InstanceName, MapBounds};
use crate::queries;
use actix_web::{get, post, web, Error, HttpResponse};

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
        queries::find_all_spawnpoints(&conn)
    })
    .await?
    .map_err(actix_web::error::ErrorInternalServerError)?;
    let markers = build_spawnpoints(&all_spawnpoints);
    Ok(HttpResponse::Ok().json(markers))
}

#[post("/spawnpoints")]
async fn spawnpoints(
    pool: web::Data<DbPool>,
    payload: web::Json<MapBounds>,
) -> Result<HttpResponse, Error> {
    let spawnpoints = web::block(move || {
        let conn = pool.get()?;
        queries::find_spawnpoints(&conn, &payload)
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
        queries::find_all_gyms(&conn)
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
        queries::find_all_pokestops(&conn)
    })
    .await?
    .map_err(actix_web::error::ErrorInternalServerError)?;
    let markers = build_pokestops(&pokestops);
    Ok(HttpResponse::Ok().json(markers))
}

#[get("/instances")]
async fn instances(pool: web::Data<DbPool>) -> Result<HttpResponse, Error> {
    let instances = web::block(move || {
        let conn = pool.get()?;
        queries::find_all_instances(&conn)
    })
    .await?
    .map_err(actix_web::error::ErrorInternalServerError)?;
    Ok(HttpResponse::Ok().json(instances))
}

#[post("/quest_generation")]
async fn quest_generation(
    pool: web::Data<DbPool>,
    payload: web::Json<InstanceName>,
) -> Result<HttpResponse, Error> {
    let instance_route = web::block(move || {
        let conn = pool.get()?;

        let instance = queries::get_instance_route(&conn, payload.name.to_string())?;

        let data: InstanceData =
            serde_json::from_str(instance.data.as_str()).expect("JSON was not well-formatted");

        let mut string: String = "".to_string();
        for i in data.area[0].iter() {
            string = string + &i.lat.to_string() + " " + &i.lon.to_string() + ",";
        };
        string = string.trim_end_matches(",").to_string();
        
        queries::get_pokestops_in_area(&conn, string)
    })
    .await?
    .map_err(actix_web::error::ErrorInternalServerError)?;
    Ok(HttpResponse::Ok().json(instance_route))
}
