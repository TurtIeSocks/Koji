use super::DbPool;

use crate::earth_distance::EarthDistance;
use crate::marker_gen::{build_gyms, build_pokestops, build_spawnpoints};
use crate::models::{InstanceData, InstanceName, MapBounds};
use crate::queries;
use crate::routing::solve;

use actix_web::{get, post, web, Error, HttpResponse};
use ndarray::Array2;
use petal_clustering::{Fit, Optics};

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

#[post("/specific_pokestops")]
async fn specific_pokestops(
    pool: web::Data<DbPool>,
    payload: web::Json<InstanceName>,
) -> Result<HttpResponse, Error> {
    println!("{:?}", payload.name);
    let stops = web::block(move || {
        let conn = pool.get()?;
        let instance = queries::get_instance_route(&conn, &payload.name)?;

        let data: InstanceData =
            serde_json::from_str(instance.data.as_str()).expect("JSON was not well-formatted");

        let mut string: String = "".to_string();
        for i in data.area[0].iter() {
            string = string + &i.lat.to_string() + " " + &i.lon.to_string() + ",";
        }
        string = string.trim_end_matches(",").to_string();

        queries::get_pokestops_in_area(&conn, string)
    })
    .await?
    .map_err(actix_web::error::ErrorInternalServerError)?;
    let markers = build_pokestops(&stops);

    Ok(HttpResponse::Ok().json(markers))
}

#[post("/quest_generation")]
async fn quest_generation(
    pool: web::Data<DbPool>,
    payload: web::Json<InstanceName>,
) -> Result<HttpResponse, Error> {
    let radius = payload.radius.clone();
    let generations = payload.generations.clone();
    let name = payload.name.clone();

    let instance_stops = web::block(move || {
        let conn = pool.get()?;

        let instance = queries::get_instance_route(&conn, &name)?;

        let data: InstanceData =
            serde_json::from_str(instance.data.as_str()).expect("JSON was not well-formatted");

        let mut string: String = "".to_string();
        for i in data.area[0].iter() {
            string = string + &i.lat.to_string() + " " + &i.lon.to_string() + ",";
        }
        string = string.trim_end_matches(",").to_string();

        queries::get_pokestops_in_area(&conn, string)
    })
    .await?
    .map_err(actix_web::error::ErrorInternalServerError)?;

    println!("{}", instance_stops.len());

    let mut data = Vec::new();

    let lat_lon_array: Vec<[f64; 2]> = instance_stops.iter().map(|p| [p.lat, p.lon]).collect();

    let ncols = lat_lon_array.first().map_or(0, |row| row.len());
    let mut nrows = 0;

    for i in 0..lat_lon_array.len() {
        data.extend_from_slice(&lat_lon_array[i]);
        nrows += 1;
    }

    let array = Array2::from_shape_vec((nrows, ncols), data).unwrap();

    let clustering =
        Optics::<f64, EarthDistance>::new(radius, 1, EarthDistance::default()).fit(&array);

    let mut services = Vec::<[f64; 2]>::new();

    println!("Clustering\n{:?}\n", clustering.0.len());
    for i in clustering.0.iter() {
        let mut sum = [0.0, 0.0];

        let mut count = 0.0;
        for j in i.1.iter() {
            count += 1.0;
            sum[0] += lat_lon_array[*j][0];
            sum[1] += lat_lon_array[*j][1];
        }
        services.push([sum[0] / count, sum[1] / count]);
    }
    let solution = solve(services, generations);

    let locations: Vec<(f64, f64)> = solution.tours[0]
        .stops
        .iter()
        .map(|p| p.clone().to_point().location.to_lat_lng())
        .collect();
    Ok(HttpResponse::Ok().json(locations))
}
