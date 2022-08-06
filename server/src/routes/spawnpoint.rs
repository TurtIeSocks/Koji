use super::*;
use crate::models::{
    api::{MapBounds, RouteGeneration},
    scanner::InstanceData,
};
use crate::queries::{instance::query_instance_route, spawnpoint::*};
use crate::utils::pixi_marker::pixi_spawnpoints;

#[get("/api/spawnpoint/all")]
async fn all(pool: web::Data<DbPool>) -> Result<HttpResponse, Error> {
    let all_spawnpoints = web::block(move || {
        let conn = pool.get()?;
        query_all_spawnpoints(&conn)
    })
    .await?
    .map_err(actix_web::error::ErrorInternalServerError)?;
    let all_spawnpoints = pixi_spawnpoints(&all_spawnpoints);
    Ok(HttpResponse::Ok().json(all_spawnpoints))
}

#[post("/api/spawnpoints/bound")]
async fn bound(
    pool: web::Data<DbPool>,
    payload: web::Json<MapBounds>,
) -> Result<HttpResponse, Error> {
    let bound_spawnpoints = web::block(move || {
        let conn = pool.get()?;
        query_bound_spawnpoints(&conn, &payload)
    })
    .await?
    .map_err(actix_web::error::ErrorInternalServerError)?;
    let bound_spawnpoints = pixi_spawnpoints(&bound_spawnpoints);
    Ok(HttpResponse::Ok().json(bound_spawnpoints))
}

#[post("/api/spawnpoint/bootstrap")]
async fn bootstrap(
    pool: web::Data<DbPool>,
    payload: web::Json<RouteGeneration>,
) -> Result<HttpResponse, Error> {
    let bs_name = payload.name.clone();
    let bs_radius = payload.radius.clone();
    let bs_generations = payload.generations.clone();
    println!(
        "Name: {}, Radius: {}, Generations: {}",
        bs_name, bs_radius, bs_generations
    );

    let instance = web::block(move || {
        let conn = pool.get()?;

        query_instance_route(&conn, &bs_name)
    })
    .await?
    .map_err(actix_web::error::ErrorInternalServerError)?;

    let data: InstanceData =
        serde_json::from_str(instance.data.as_str()).expect("JSON was not well-formatted");

    // let circles = generate_circles(data.area[0].clone());
    Ok(HttpResponse::Ok().json(data))
}
