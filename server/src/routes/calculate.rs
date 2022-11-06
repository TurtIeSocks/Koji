use super::*;
use std::collections::VecDeque;
use time::Duration;
use travelling_salesman;

use crate::models::{
    api::{CustomError, RouteGeneration},
    scanner::{GenericData, InstanceData},
};
use crate::queries::{gym, instance::query_instance_route, pokestop, spawnpoint};
use crate::utils::{
    bootstrapping::generate_circles,
    // convert::text,
    project_points::project_points,
    // routing::solve;
    response,
    to_array::{coord_to_array, data_to_array},
};

#[post("/bootstrap")]
async fn bootstrap(
    conn: web::Data<DatabaseConnection>,
    scanner_type: web::Data<String>,
    payload: web::Json<RouteGeneration>,
) -> Result<HttpResponse, Error> {
    let scanner_type = scanner_type.as_ref();

    let RouteGeneration {
        instance,
        radius,
        area,
        return_type,
        generations: _generations,
        devices: _devices,
        data_points: _data_points,
        min_points: _min_points,
        fast: _fast,
    } = payload.into_inner();
    let instance = instance.unwrap_or("".to_string());
    let radius = radius.unwrap_or(70.0);
    let area = area.unwrap_or(vec![]);
    let return_type = return_type.unwrap_or("json".to_string());

    println!(
        "\n[BOOTSTRAP] Mode: Bootstrap, Radius: {}\nScanner Type: {}, Instance: {}, Custom Area: {}",
        radius,
        scanner_type,
        instance,
        area.len() > 0
    );

    if !scanner_type.eq("rdm") && area.len() == 0 {
        return Ok(HttpResponse::BadRequest().json(CustomError {
            message: "no_area_provided_and_invalid_scanner_type".to_string(),
        }));
    }
    if area.len() == 0 && instance.is_empty() {
        return Ok(HttpResponse::BadRequest().json(CustomError {
            message: "no_area_and_empty_instance".to_string(),
        }));
    }

    let area = if area.len() > 0 {
        area
    } else if !instance.is_empty() && scanner_type.eq("rdm") {
        let instance =
            web::block(move || async move { query_instance_route(&conn, &instance).await })
                .await?
                .await
                .map_err(actix_web::error::ErrorInternalServerError)?;

        let data: InstanceData =
            serde_json::from_str(instance.data.as_str()).expect("JSON was not well-formatted");
        coord_to_array(data.area[0].clone())
    } else {
        vec![]
    };

    let circles = generate_circles(area, radius);

    println!("[BOOTSTRAP] Returning {} circles\n", circles[0].len());
    Ok(response::send(
        circles
            .iter()
            .map(|[lat, lon]| [*lat as f32, *lon as f32])
            .collect::<Vec<[f32; 2]>>(),
        return_type,
    ))
}

#[post("/{mode}/{category}")]
async fn cluster(
    conn: web::Data<DatabaseConnection>,
    scanner_type: web::Data<String>,
    url: actix_web::web::Path<(String, String)>,
    payload: web::Json<RouteGeneration>,
) -> Result<HttpResponse, Error> {
    let (mode, category) = url.into_inner();
    let category_2 = category.clone();
    let scanner_type = scanner_type.as_ref();

    let RouteGeneration {
        instance,
        radius,
        generations,
        devices,
        area,
        data_points,
        min_points,
        fast,
        return_type,
    } = payload.into_inner();
    let instance = instance.unwrap_or("".to_string());
    let radius = radius.unwrap_or(70.0);
    let generations = generations.unwrap_or(0);
    let devices = devices.unwrap_or(1);
    let area = area.unwrap_or(vec![]);
    let data_points = data_points.unwrap_or(vec![]);
    let min_points = min_points.unwrap_or(1);
    let fast = fast.unwrap_or(false);
    let return_type = return_type.unwrap_or("json".to_string());

    println!(
        "\n[{}] Radius: {}, Generations: {}, Devices: {}\nInstance: {}, Using Area: {}, Manual Data Points: {}",
        mode.to_uppercase(), radius, generations, devices, instance, area.len() > 0, data_points.len()
    );

    if !scanner_type.eq("rdm") && area.len() == 0 {
        return Ok(HttpResponse::BadRequest().json(CustomError {
            message: "no_area_provided_and_invalid_scanner_type".to_string(),
        }));
    }
    if area.len() == 0 && instance.is_empty() {
        return Ok(HttpResponse::BadRequest().json(CustomError {
            message: "no_area_and_empty_instance".to_string(),
        }));
    }

    let data_points = if data_points.len() > 0 {
        data_points
    } else {
        web::block(move || async move {
            let instance = if instance.is_empty() {
                None
            } else {
                Some(query_instance_route(&conn, &instance).await?)
            };
            let area = if instance.is_some() && area.len() == 0 {
                let instance_data: InstanceData =
                    serde_json::from_str(instance.unwrap().data.as_str())
                        .expect("JSON was not well-formatted");
                coord_to_array(instance_data.area[0].clone())
            } else {
                area
            };
            if area.len() > 1 {
                if category == "gym" {
                    gym::area(&conn, &area).await
                } else if category == "pokestop" {
                    pokestop::area(&conn, &area).await
                } else {
                    spawnpoint::area(&conn, &area).await
                }
            } else {
                Ok(Vec::<GenericData>::new())
            }
        })
        .await?
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?
    };
    println!(
        "[{}] Found Data Points: {}",
        mode.to_uppercase(),
        data_points.len()
    );

    let (clusters, biggest) = project_points(
        data_to_array(data_points),
        radius - 1.,
        min_points,
        fast,
        category_2,
    );
    println!("[{}] Clusters: {}", mode.to_uppercase(), clusters.len());

    if mode.eq("cluster") {
        return Ok(response::send(
            clusters
                .iter()
                .map(|[lat, lon]| [*lat as f32, *lon as f32])
                .collect::<Vec<[f32; 2]>>(),
            return_type,
        ));
    }

    println!("Routing for {}seconds...", generations);
    let tour = travelling_salesman::simulated_annealing::solve(
        &clusters
            .iter()
            .map(|[x, y]| (*x, *y))
            .collect::<Vec<(f64, f64)>>()[0..clusters.len()],
        Duration::seconds(if generations > 0 {
            generations as i64
        } else {
            ((clusters.len() / 100) as i64 + 1) * if fast { 1 } else { 2 }
        }),
    );

    let mut final_clusters = VecDeque::<[f32; 2]>::new();

    let mut rotate: usize = 0;
    for (i, index) in tour.route.iter().enumerate() {
        let [lat, lon] = clusters[*index];
        if lat == biggest[0] && lon == biggest[1] {
            rotate = i;
            println!("Found Best! {}, {} - {}", lat, lon, index);
        }
        final_clusters.push_back([lat as f32, lon as f32]);
    }
    final_clusters.rotate_left(rotate);

    // let circles = solve(clusters, generations, devices);
    // let mapped_circles: Vec<Vec<(f64, f64)>> = circles
    //     .tours
    //     .iter()
    //     .map(|p| {
    //         p.stops
    //             .iter()
    //             .map(|x| x.clone().to_point().location.to_lat_lng())
    //             .collect()
    //     })
    //     .collect();

    println!(
        "[{}] Returning {} clusters {} routes\n ",
        mode.to_uppercase(),
        clusters.len(),
        (clusters.len() / 100) as i64,
    );
    Ok(response::send(
        final_clusters.iter().map(|x| *x).collect::<Vec<[f32; 2]>>(),
        return_type,
    ))
}
