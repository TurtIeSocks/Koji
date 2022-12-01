use super::*;
use geo::{HaversineDistance, Point};
use std::collections::VecDeque;
use std::time::Instant;
use time::Duration;
use travelling_salesman;

use crate::entities::sea_orm_active_enums::Type;
use crate::models::api::Stats;
use crate::models::{api::Args, scanner::GenericData, KojiDb};
use crate::models::{CustomError, SingleVec};
use crate::queries::{area, gym, instance, pokestop, spawnpoint};
use crate::utils::convert::collection;
use crate::utils::drawing::clustering_2::brute_force;
use crate::utils::{
    convert::{feature, normalize, vector},
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
        benchmark_mode,
        routing_time: _routing_time,
        return_type,
        generations: _generations,
        devices: _devices,
        data_points: _data_points,
        min_points: _min_points,
        fast: _fast,
    } = payload.into_inner().log("Bootstrap");
    let instance = instance.unwrap_or("".to_string());
    let radius = radius.unwrap_or(70.0);
    let (area, default_return_type) = normalize::area_input(area);
    let return_type = get_return_type(return_type, default_return_type);
    let benchmark_mode = benchmark_mode.unwrap_or(false);

    if area.features.is_empty() && instance.is_empty() {
        return Ok(HttpResponse::BadRequest().json(CustomError {
            message: "no_area_and_empty_instance".to_string(),
        }));
    }

    let area = if area.features.is_empty() && !instance.is_empty() {
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

    let mut stats = Stats::new();

    let time = Instant::now();

    let features: Vec<Feature> = area
        .into_iter()
        .map(|sub_area| bootstrapping::as_geojson(sub_area, radius, &mut stats))
        .collect();

    stats.cluster_time = time.elapsed().as_secs_f32();

    Ok(response::send(
        collection::from_features(features),
        return_type,
        stats,
        benchmark_mode,
    ))
}

#[post("/{mode}/{category}")]
async fn cluster(
    conn: web::Data<KojiDb>,
    scanner_type: web::Data<String>,
    url: actix_web::web::Path<(String, String)>,
    payload: web::Json<Args>,
) -> Result<HttpResponse, Error> {
    let (mode, category) = url.into_inner();
    let scanner_type = scanner_type.as_ref().clone();

    let Args {
        instance,
        radius,
        generations,
        routing_time,
        devices: _devices,
        area,
        data_points,
        min_points,
        fast,
        return_type,
        benchmark_mode,
    } = payload.into_inner().log(&mode);
    let instance = instance.unwrap_or("".to_string());
    let radius = radius.unwrap_or(70.0);
    let generations = generations.unwrap_or(1);
    let routing_time = routing_time.unwrap_or(1);
    let data_points = normalize::data_points(data_points);
    let min_points = min_points.unwrap_or(1);
    let fast = fast.unwrap_or(true);
    let (area, default_return_type) = normalize::area_input(area);
    let return_type = get_return_type(return_type, default_return_type);
    let benchmark_mode = benchmark_mode.unwrap_or(false);

    if area.features.is_empty() && instance.is_empty() {
        return Ok(HttpResponse::BadRequest().json(CustomError {
            message: "no_area_and_empty_instance".to_string(),
        }));
    }

    let mut stats = Stats::new();

    println!("Area {}", area.features.len());

    let temp_instance = instance.clone();
    let area = if !temp_instance.is_empty() {
        if scanner_type.eq("rdm") {
            instance::route(&conn.data_db, &instance).await
        } else {
            area::route(&conn.unown_db.as_ref().unwrap(), &instance).await
        }
    } else {
        Ok(area)
    }
    .map_err(actix_web::error::ErrorInternalServerError)?;

    let data_points = if !data_points.is_empty() {
        data_points
            .iter()
            .map(|p| GenericData::new("".to_string(), p[0], p[1]))
            .collect()
    } else {
        let temp_area = area.clone();
        if !temp_area.features.is_empty() {
            if category == "gym" {
                gym::area(&conn.data_db, &area).await
            } else if category == "pokestop" {
                pokestop::area(&conn.data_db, &area).await
            } else {
                spawnpoint::area(&conn.data_db, &area).await
            }
        } else {
            Ok(vec![])
        }
        .map_err(actix_web::error::ErrorInternalServerError)?
    };
    println!(
        "[{}] Found Data Points: {}",
        mode.to_uppercase(),
        data_points.len()
    );

    let clusters: SingleVec = if fast {
        project_points(
            vector::from_generic_data(data_points),
            radius,
            min_points,
            &mut stats,
        )
    } else {
        area.into_iter()
            .flat_map(|feature| {
                brute_force(
                    data_points.clone(),
                    bootstrapping::as_vec(feature, radius, &mut stats),
                    radius,
                    min_points,
                    generations,
                    &mut stats,
                )
            })
            .collect()
    };

    if mode.eq("cluster") || clusters.is_empty() || routing_time == 0 {
        return Ok(response::send(
            collection::from_feature(feature::from_single_vector(
                clusters,
                Some(Type::CirclePokemon),
            )),
            return_type,
            stats,
            benchmark_mode,
        ));
    }

    println!("Routing for {}seconds...", routing_time);
    let tour = travelling_salesman::simulated_annealing::solve(
        &clusters
            .iter()
            .map(|[x, y]| (*x, *y))
            .collect::<Vec<(f64, f64)>>()[0..clusters.len()],
        Duration::seconds(if routing_time > 0 {
            routing_time
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
        if lat == stats.best_cluster[0] && lon == stats.best_cluster[1] {
            rotate_count = i;
            println!("Found Best! {}, {} - {}", lat, lon, index);
        }
        final_clusters.push_back([lat, lon]);
    }
    final_clusters.rotate_left(rotate_count);

    for (i, point) in final_clusters.clone().into_iter().enumerate() {
        let point = Point::new(point[1], point[0]);
        let point2 = if i == final_clusters.len() - 1 {
            Point::new(final_clusters[0][1], final_clusters[0][0])
        } else {
            Point::new(final_clusters[i + 1][1], final_clusters[i + 1][0])
        };
        let distance = point.haversine_distance(&point2);
        stats.total_distance += distance;
        if distance > stats.longest_distance {
            stats.longest_distance = distance;
        }
    }

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

    Ok(response::send(
        collection::from_feature(feature::from_single_vector(
            final_clusters.into(),
            Some(Type::CirclePokemon),
        )),
        return_type,
        stats,
        benchmark_mode,
    ))
}
