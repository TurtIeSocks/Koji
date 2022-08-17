use super::*;
use crate::cpp::bridge::cpp_cluster;
use crate::models::{api::RouteGeneration, scanner::InstanceData};
use crate::queries::{gym, instance::query_instance_route, pokestop, spawnpoint};
use crate::utils::bootstrapping::generate_circles;
use crate::utils::routing::solve;

#[post("/bootstrap")]
async fn bootstrap(
    pool: web::Data<DbPool>,
    payload: web::Json<RouteGeneration>,
) -> Result<HttpResponse, Error> {
    let instance = payload.instance.clone().unwrap_or_else(|| "".to_string());
    let radius = payload.radius.clone().unwrap_or_else(|| 0.0);
    if instance == "" || radius == 0.0 {
        return Ok(HttpResponse::Ok().json(""));
    }
    let instance = web::block(move || {
        let conn = pool.get()?;

        query_instance_route(&conn, &instance)
    })
    .await?
    .map_err(actix_web::error::ErrorInternalServerError)?;

    let data: InstanceData =
        serde_json::from_str(instance.data.as_str()).expect("JSON was not well-formatted");

    let circles = generate_circles(data.area[0].clone(), radius);
    Ok(HttpResponse::Ok().json([circles]))
}

#[post("/{mode}/{category}")]
async fn cluster(
    pool: web::Data<DbPool>,
    info: actix_web::web::Path<(String, String)>,
    payload: web::Json<RouteGeneration>,
) -> Result<HttpResponse, Error> {
    let name = payload.instance.clone().unwrap_or_else(|| "".to_string());
    let radius = payload.radius.clone().unwrap_or_else(|| 1.0);
    let generations = payload.generations.clone().unwrap_or_else(|| 1);
    let (mode, category) = info.into_inner();
    let devices = payload.devices.clone().unwrap_or_else(|| 1);

    println!(
        "Name: {}, Radius: {}, Generations: {}, Mode: {}, Devices: {}",
        name, radius, generations, mode, devices,
    );

    if name == "" || radius == 0.0 || generations == 0 {
        return Ok(HttpResponse::Ok().json(""));
    }

    let x: String = category.clone();
    let y: String = x.clone();
    let raw_data = web::block(move || {
        let conn = pool.get()?;
        let instance = query_instance_route(&conn, &name)?;
        let data: InstanceData =
            serde_json::from_str(instance.data.as_str()).expect("JSON was not well-formatted");
        if x == "gym" {
            gym::area(&conn, &data.area[0])
        } else if x == "pokestop" {
            pokestop::area(&conn, &data.area[0])
        } else {
            spawnpoint::area(&conn, &data.area[0])
        }
    })
    .await?
    .map_err(actix_web::error::ErrorInternalServerError)?;

    println!("{}: {}", y, raw_data.len());

    let lat_lon_array: Vec<[f64; 2]> = raw_data.iter().map(|p| [p.lat, p.lon]).collect();
    let clusters = cpp_cluster(lat_lon_array, 98650. / radius);

    if mode.as_str() == "cluster" {
        return Ok(HttpResponse::Ok().json([clusters]));
    }
    let clusters = solve(clusters, generations, devices);
    println!("Tours: {}", clusters.tours.len());
    let clusters: Vec<Vec<(f64, f64)>> = clusters
        .tours
        .iter()
        .map(|p| {
            p.stops
                .iter()
                .map(|x| x.clone().to_point().location.to_lat_lng())
                .collect()
        })
        .collect();
    Ok(HttpResponse::Ok().json(clusters))
}
