use super::*;
use crate::models::api::MapBounds;
use crate::queries::spawnpoint::*;
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
