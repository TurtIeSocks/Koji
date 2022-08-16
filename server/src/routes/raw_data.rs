use super::*;
use crate::models::{
    api::{MapBounds, RouteGeneration},
    scanner::InstanceData,
};
use crate::queries::{gym, instance::query_instance_route, pokestop, spawnpoint};
use crate::utils::pixi_marker::pixi_marker;

#[get("/all/{category}")]
async fn all(
    pool: web::Data<DbPool>,
    category: actix_web::web::Path<String>,
) -> Result<HttpResponse, Error> {
    let str = category.into_inner().clone();
    let copy = str.clone();
    let all_data = web::block(move || {
        let conn = pool.get()?;
        if str == "gym" {
            gym::all(&conn)
        } else if str == "pokestop" {
            pokestop::all(&conn)
        } else {
            spawnpoint::all(&conn)
        }
    })
    .await?
    .map_err(actix_web::error::ErrorInternalServerError)?;
    let all_data = pixi_marker(&all_data, &copy);
    Ok(HttpResponse::Ok().json(all_data))
}

#[post("/bound/{category}")]
async fn bound(
    pool: web::Data<DbPool>,
    payload: web::Json<MapBounds>,
    category: actix_web::web::Path<String>,
) -> Result<HttpResponse, Error> {
    let str = category.into_inner();
    let copy = str.clone();
    let bound_gyms = web::block(move || {
        let conn = pool.get()?;
        if str == "gym" {
            gym::bound(&conn, &payload)
        } else if str == "pokestop" {
            pokestop::bound(&conn, &payload)
        } else {
            spawnpoint::bound(&conn, &payload)
        }
    })
    .await?
    .map_err(actix_web::error::ErrorInternalServerError)?;
    let bound_spawnpoints = pixi_marker(&bound_gyms, &copy);
    Ok(HttpResponse::Ok().json(bound_spawnpoints))
}

#[post("/area/{category}")]
async fn area(
    pool: web::Data<DbPool>,
    payload: web::Json<RouteGeneration>,
    category: actix_web::web::Path<String>,
) -> Result<HttpResponse, Error> {
    let str = category.into_inner();
    let copy = str.clone();
    let gyms = web::block(move || {
        let conn = pool.get()?;
        let instance = query_instance_route(&conn, &payload.instance)?;
        let data: InstanceData =
            serde_json::from_str(instance.data.as_str()).expect("JSON was not well-formatted");
        if str == "gym" {
            gym::area(&conn, &data.area[0])
        } else if str == "pokestop" {
            pokestop::area(&conn, &data.area[0])
        } else {
            spawnpoint::area(&conn, &data.area[0])
        }
    })
    .await?
    .map_err(actix_web::error::ErrorInternalServerError)?;

    let markers = pixi_marker(&gyms, &copy);

    Ok(HttpResponse::Ok().json(markers))
}
