use super::*;
use crate::clustering::bridge::cpp_cluster;
use crate::models::{api::RouteGeneration, scanner::InstanceData};
use crate::queries::{instance::query_instance_route, pokestop::*};
use crate::utils::pixi_marker::pixi_pokestops;
use crate::utils::routing::solve;

#[get("/api/pokestop/all")]
async fn all(pool: web::Data<DbPool>) -> Result<HttpResponse, Error> {
    let pokestops = web::block(move || {
        let conn = pool.get()?;
        query_all_pokestops(&conn)
    })
    .await?
    .map_err(actix_web::error::ErrorInternalServerError)?;
    let pokestops = pixi_pokestops(&pokestops);
    Ok(HttpResponse::Ok().json(pokestops))
}

#[post("/api/pokestop/area")]
async fn area(
    pool: web::Data<DbPool>,
    payload: web::Json<RouteGeneration>,
) -> Result<HttpResponse, Error> {
    let stops = web::block(move || {
        let conn = pool.get()?;
        let instance = query_instance_route(&conn, &payload.instance)?;
        let data: InstanceData =
            serde_json::from_str(instance.data.as_str()).expect("JSON was not well-formatted");

        let mut string: String = "".to_string();
        for i in data.area[0].iter() {
            string = string + &i.lat.to_string() + " " + &i.lon.to_string() + ",";
        }
        string = string.trim_end_matches(",").to_string();
        query_area_pokestops(&conn, string)
    })
    .await?
    .map_err(actix_web::error::ErrorInternalServerError)?;

    let markers = pixi_pokestops(&stops);

    Ok(HttpResponse::Ok().json(markers))
}

#[post("/api/pokestop/route")]
async fn route(
    pool: web::Data<DbPool>,
    payload: web::Json<RouteGeneration>,
) -> Result<HttpResponse, Error> {
    let name = payload.instance.clone();
    let radius = payload.radius.clone();
    let generations = payload.generations.clone();
    let mode = payload.mode.clone();
    println!(
        "Name: {}, Radius: {}, Generations: {}",
        name, radius, generations
    );

    let instance_stops = web::block(move || {
        let conn = pool.get()?;

        let instance = query_instance_route(&conn, &name)?;

        let data: InstanceData =
            serde_json::from_str(instance.data.as_str()).expect("JSON was not well-formatted");

        let mut string: String = "".to_string();
        for i in data.area[0].iter() {
            string = string + &i.lat.to_string() + " " + &i.lon.to_string() + ",";
        }
        string = string.trim_end_matches(",").to_string();

        query_area_pokestops(&conn, string)
    })
    .await?
    .map_err(actix_web::error::ErrorInternalServerError)?;

    println!("Pokestops: {}", instance_stops.len());

    let lat_lon_array: Vec<[f64; 2]> = instance_stops.iter().map(|p| [p.lat, p.lon]).collect();
    let clusters = cpp_cluster(lat_lon_array, 98650. / radius);

    if mode == "cluster" {
        return Ok(HttpResponse::Ok().json(clusters));
    }
    let clusters = solve(clusters, generations, radius * 1000.);

    let clusters: Vec<(f64, f64)> = clusters.tours[0]
        .stops
        .iter()
        .map(|p| p.clone().to_point().location.to_lat_lng())
        .collect();
    Ok(HttpResponse::Ok().json(clusters))
}
