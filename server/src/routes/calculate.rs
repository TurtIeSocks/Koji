use super::*;

use geo::{Coordinate, HaversineDistance, Point};
use geojson::{Geometry, Value};
use std::collections::{HashSet, VecDeque};
use std::time::Instant;
use time::Duration;
use travelling_salesman;

use crate::models::BBox;
use crate::{
    entities::sea_orm_active_enums::Type,
    models::{
        api::{Args, ArgsUnwrapped, Stats},
        scanner::GenericData,
        CustomError, KojiDb, SingleVec,
    },
    queries::{area, gym, instance, pokestop, spawnpoint},
    utils::{
        clustering,
        convert::{collection, feature, vector},
        drawing::{bootstrapping, project_points},
        response,
    },
};

#[post("/bootstrap")]
async fn bootstrap(
    conn: web::Data<KojiDb>,
    scanner_type: web::Data<String>,
    payload: web::Json<Args>,
) -> Result<HttpResponse, Error> {
    let scanner_type = scanner_type.as_ref();

    let ArgsUnwrapped {
        area,
        benchmark_mode,
        instance,
        radius,
        return_type,
        ..
    } = payload.into_inner().init(Some("bootstrap"));

    if area.features.is_empty() && instance.is_empty() {
        return Ok(HttpResponse::BadRequest().json(CustomError {
            message: "no_area_and_empty_instance".to_string(),
        }));
    }

    let area = if area.features.is_empty() && !instance.is_empty() {
        if scanner_type.eq("rdm") {
            instance::route(&conn.data_db, &instance).await
        } else {
            area::route(&conn.unown_db.as_ref().unwrap(), &instance).await
        }
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
        instance,
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
    let scanner_type = scanner_type.as_ref();

    let ArgsUnwrapped {
        area,
        benchmark_mode,
        data_points,
        fast,
        generations,
        instance,
        min_points,
        radius,
        return_type,
        routing_time,
        only_unique,
        ..
    } = payload.into_inner().init(Some(&mode));

    if area.features.is_empty() && instance.is_empty() && data_points.is_empty() {
        return Ok(HttpResponse::BadRequest().json(CustomError {
            message: "no_area_instance_data_points".to_string(),
        }));
    }

    let mut stats = Stats::new();

    let area = if !instance.is_empty() {
        if scanner_type.eq("rdm") {
            instance::route(&conn.data_db, &instance).await
        } else {
            area::route(&conn.unown_db.as_ref().unwrap(), &instance).await
        }
    } else if !data_points.is_empty() {
        let polygon = BBox::new(Some(
            &data_points
                .iter()
                .map(|p| Coordinate { x: p[1], y: p[0] })
                .collect(),
        ))
        .get_poly();
        Ok(FeatureCollection {
            bbox: None,
            features: vec![Feature {
                geometry: Some(Geometry {
                    value: Value::Polygon(polygon),
                    bbox: None,
                    foreign_members: None,
                }),
                ..Default::default()
            }],
            foreign_members: None,
        })
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
        if !area.features.is_empty() {
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

    stats.total_points = data_points.len();

    let clusters: SingleVec = if fast {
        project_points::project_points(
            vector::from_generic_data(data_points),
            radius,
            min_points,
            &mut stats,
        )
    } else {
        area.into_iter()
            .flat_map(|feature| {
                clustering::brute_force(
                    &data_points,
                    bootstrapping::as_vec(feature, radius, &mut stats),
                    radius,
                    min_points,
                    generations,
                    &mut stats,
                    only_unique,
                )
            })
            .collect()
    };

    if mode.eq("cluster") || clusters.is_empty() {
        return Ok(response::send(
            collection::from_feature(feature::from_single_vector(
                clusters,
                Some(&Type::CirclePokemon),
            )),
            return_type,
            stats,
            benchmark_mode,
            instance,
        ));
    }

    println!("Routing for {} seconds...", routing_time);
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

    let mut hash = HashSet::<usize>::new();

    for (i, index) in tour.route.into_iter().enumerate() {
        if hash.contains(&index) {
            continue;
        } else {
            hash.insert(index);
        }
        let [lat, lon] = clusters[index];
        if stats.best_clusters.len() >= 1
            && lat == stats.best_clusters[0][0]
            && lon == stats.best_clusters[0][1]
        {
            rotate_count = i;
            println!("Found Best! {}, {} - {}", lat, lon, index);
        }
        final_clusters.push_back([lat, lon]);
    }
    final_clusters.rotate_left(rotate_count);
    stats.total_distance = 0.;
    stats.longest_distance = 0.;

    for (i, point) in final_clusters.iter().enumerate() {
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
            Some(&Type::CirclePokemon),
        )),
        return_type,
        stats,
        benchmark_mode,
        instance,
    ))
}
