use super::*;
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
        let instance = query_instance_route(&conn, &payload.name)?;
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
    let name = payload.name.clone();
    let radius = payload.radius.clone();
    let generations = payload.generations.clone();
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

    println!("{}", instance_stops.len());

    // let mut data = Vec::new();

    let lat_lon_array: Vec<[f64; 2]> = instance_stops.iter().map(|p| [p.lat, p.lon]).collect();

    // let ncols = lat_lon_array.first().map_or(0, |row| row.len());
    // let mut nrows = 0;

    // for i in 0..lat_lon_array.len() {
    //     data.extend_from_slice(&lat_lon_array[i]);
    //     nrows += 1;
    // }

    // let array = Array2::from_shape_vec((nrows, ncols), data).unwrap();

    // let clustering =
    //     Optics::<f64, EarthDistance>::new(radius, 1, EarthDistance::default()).fit(&array);

    // let mut services = Vec::<[f64; 2]>::new();

    // println!("Clustering\n{:?}\n", clustering.0.len());
    // for i in clustering.0.iter() {
    //     let mut sum = [0.0, 0.0];

    //     let mut count = 0.0;
    //     for j in i.1.iter() {
    //         count += 1.0;
    //         sum[0] += lat_lon_array[*j][0];
    //         sum[1] += lat_lon_array[*j][1];
    //     }
    //     services.push([sum[0] / count, sum[1] / count]);
    // }
    let solution = solve(lat_lon_array, generations, radius * 1000.);

    let locations: Vec<(f64, f64)> = solution.tours[0]
        .stops
        .iter()
        .map(|p| p.clone().to_point().location.to_lat_lng())
        .collect();
    Ok(HttpResponse::Ok().json(locations))
}
