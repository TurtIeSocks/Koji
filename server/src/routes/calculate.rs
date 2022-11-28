use super::*;
use std::collections::VecDeque;
use time::Duration;
use travelling_salesman;

use crate::models::{api::Args, scanner::GenericData, KojiDb};
use crate::models::{CustomError, MultiVec, SingleVec};
use crate::queries::{area, gym, instance, pokestop, spawnpoint};
use crate::utils::drawing::clustering_2::brute_force;
use crate::utils::{
    convert::{normalize, vector},
    drawing::{bootstrapping, project_points::project_points},
    get_return_type, response,
};

#[post("/bootstrap")]
async fn bootstrap(
    conn: web::Data<KojiDb>,
    scanner_type: web::Data<String>,
    payload: web::Json<Args>,
) -> Result<HttpResponse, Error> {
    let scanner_type = scanner_type.as_ref().clone();

    let Args {
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
    let (area, default_return_type) = normalize::area_input(area);
    let return_type = get_return_type(return_type, default_return_type);

    println!(
        "\n[BOOTSTRAP] Mode: Bootstrap, Radius: {}\nScanner Type: {}, Instance: {}, Custom Area: {:?}",
        radius,
        scanner_type,
        instance,
        area,
    );

    if area.features.is_empty() && instance.is_empty() {
        return Ok(HttpResponse::BadRequest().json(CustomError {
            message: "no_area_and_empty_instance".to_string(),
        }));
    }

    let area = if !area.features.is_empty() {
        area
    } else if !instance.is_empty() {
        web::block(move || async move {
            if scanner_type.eq("rdm") {
                instance::route(&conn.data_db, &instance).await
            } else {
                area::route(&conn.unown_db.as_ref().unwrap(), &instance).await
            }
        })
        .await?
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?
    } else {
        area
    };

    let circles: MultiVec = area
        .into_iter()
        .map(|sub_area| bootstrapping::check(sub_area, radius))
        .collect();

    println!("[BOOTSTRAP] Returning {} circles\n", circles[0].len());
    Ok(response::send(circles, return_type))
}

#[post("/{mode}/{category}")]
async fn cluster(
    conn: web::Data<KojiDb>,
    scanner_type: web::Data<String>,
    url: actix_web::web::Path<(String, String)>,
    payload: web::Json<Args>,
) -> Result<HttpResponse, Error> {
    let (mode, category) = url.into_inner();
    let category_2 = category.clone();
    let scanner_type = scanner_type.as_ref().clone();

    let Args {
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
    let generations = generations.unwrap_or(1);
    let devices = devices.unwrap_or(1);
    let data_points = normalize::data_points(data_points);
    let min_points = min_points.unwrap_or(1);
    let fast = fast.unwrap_or(true);
    let (area, default_return_type) = normalize::area_input(area);
    let return_type = get_return_type(return_type, default_return_type);

    println!(
        "\n[{}] Radius: {}, Generations: {}, Devices: {} Min Points: {}\nInstance: {}, Using Area: {}, Manual Data Points: {}",
        mode.to_uppercase(), radius, generations, devices, min_points, instance, area.features.len() > 0, data_points.len()
    );

    if area.features.is_empty() && instance.is_empty() {
        return Ok(HttpResponse::BadRequest().json(CustomError {
            message: "no_area_and_empty_instance".to_string(),
        }));
    }

    let data_points = if !data_points.is_empty() {
        data_points
            .iter()
            .map(|p| GenericData::new("".to_string(), p[0], p[1]))
            .collect()
    } else {
        let temp_area = area.clone();
        web::block(move || async move {
            let area = if !temp_area.features.is_empty() {
                temp_area
            } else if !instance.is_empty() {
                if scanner_type.eq("rdm") {
                    instance::route(&conn.data_db, &instance).await?
                } else {
                    area::route(&conn.unown_db.as_ref().unwrap(), &instance).await?
                }
            } else {
                temp_area
            };

            if !area.features.is_empty() {
                if category == "gym" {
                    gym::area(&conn.data_db, area).await
                } else if category == "pokestop" {
                    pokestop::area(&conn.data_db, area).await
                } else {
                    spawnpoint::area(&conn.data_db, area).await
                }
            } else {
                Ok(vec![])
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

    let (clusters, biggest): (SingleVec, [f64; 2]) = if fast {
        project_points(
            vector::from_generic_data(data_points),
            radius,
            min_points,
            category_2,
        )
    } else {
        (
            area.into_iter()
                .flat_map(|feature| {
                    brute_force(
                        data_points.clone(),
                        bootstrapping::check(feature, radius),
                        radius,
                        min_points,
                        generations,
                    )
                })
                .collect(),
            [0., 0.],
        )
    };

    println!("[{}] Clusters: {}", mode.to_uppercase(), clusters.len());

    if mode.eq("cluster") || clusters.is_empty() || generations == 0 {
        return Ok(response::send(vec![clusters], return_type));
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
            ((clusters.len() as f32 / 100.) + 1.)
                .powf(if fast { 1. } else { 1.25 })
                .floor() as i64
        }),
    );

    let mut final_clusters = VecDeque::<[f64; 2]>::new();

    let mut rotate_count: usize = 0;
    for (i, index) in tour.route.into_iter().enumerate() {
        let [lat, lon] = clusters[index];
        if lat == biggest[0] && lon == biggest[1] {
            rotate_count = i;
            println!("Found Best! {}, {} - {}", lat, lon, index);
        }
        final_clusters.push_back([lat as f64, lon as f64]);
    }
    final_clusters.rotate_left(rotate_count);

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
        (clusters.len() / 100),
    );
    Ok(response::send(
        vec![final_clusters.into_iter().map(|x| x).collect::<SingleVec>()],
        return_type,
    ))
}
