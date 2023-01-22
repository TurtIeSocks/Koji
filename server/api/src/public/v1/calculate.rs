use crate::utils::request;

use super::*;

use std::{
    collections::{HashSet, VecDeque},
    time::Instant,
};

use algorithms::{bootstrapping, clustering, routing::tsp};
use geo::{HaversineDistance, Point};

use model::{
    api::{
        args::{Args, ArgsUnwrapped, Response, Stats},
        point_array::PointArray,
        FeatureHelpers, Precision, ToCollection, ToFeature,
    },
    db::{area, instance, route, sea_orm_active_enums::Type, GenericData},
    KojiDb,
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
        save_to_db,
        save_to_scanner,
        ..
    } = payload.into_inner().init(Some("bootstrap"));

    if area.features.is_empty() && instance.is_empty() {
        return Ok(
            HttpResponse::BadRequest().json(Response::send_error("no_area_and_empty_instance"))
        );
    }

    let area = utils::create_or_find_collection(&instance, scanner_type, &conn, area, &vec![])
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;

    let mut stats = Stats::new();

    let time = Instant::now();

    let mut features: Vec<Feature> = area
        .into_iter()
        .map(|sub_area| bootstrapping::as_geojson(sub_area, radius, &mut stats))
        .collect();

    stats.cluster_time = time.elapsed().as_secs_f32() as Precision;

    for feat in features.iter_mut() {
        if !feat.contains_property("__name") && !instance.is_empty() {
            feat.set_property("__name", instance.clone());
        }
        feat.set_property("__type", Type::CircleSmartPokemon.to_string());
        if save_to_db {
            route::Query::upsert_from_collection(
                &conn.koji_db,
                feat.clone().to_collection(Some(instance.clone()), None),
                true,
                true,
            )
            .await
            .map_err(actix_web::error::ErrorInternalServerError)?;
        }
        if save_to_scanner {
            if scanner_type == "rdm" {
                instance::Query::upsert_from_collection(
                    &conn.data_db,
                    feat.clone().to_collection(Some(instance.clone()), None),
                    true,
                )
                .await
            } else if let Some(conn) = conn.unown_db.as_ref() {
                area::Query::upsert_from_collection(conn, feat.clone().to_collection(None, None))
                    .await
            } else {
                Err(DbErr::Custom(
                    "Scanner not configured correctly".to_string(),
                ))
            }
            .map_err(actix_web::error::ErrorInternalServerError)?;
        }
    }
    if save_to_scanner {
        request::update_project_api(&conn.koji_db, Some(scanner_type))
            .await
            .map_err(actix_web::error::ErrorInternalServerError)?;
    }

    Ok(utils::response::send(
        features.to_collection(Some(instance.clone()), None),
        return_type,
        Some(stats),
        benchmark_mode,
        Some(instance),
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
        instance,
        min_points,
        radius,
        return_type,
        routing_time,
        only_unique,
        save_to_db,
        save_to_scanner,
        last_seen,
        route_chunk_size,
        ..
    } = payload.into_inner().init(Some(&mode));

    if area.features.is_empty() && instance.is_empty() && data_points.is_empty() {
        return Ok(
            HttpResponse::BadRequest().json(Response::send_error("no_area_instance_data_points"))
        );
    }

    let mut stats = Stats::new();
    let enum_type = if category == "gym" {
        Type::CircleSmartRaid
    } else if category == "pokestop" {
        Type::ManualQuest
    } else {
        Type::CircleSmartPokemon
    };

    let area = utils::create_or_find_collection(&instance, scanner_type, &conn, area, &data_points)
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;

    let data_points = if !data_points.is_empty() {
        data_points
            .iter()
            .map(|p| GenericData::new("".to_string(), p[0], p[1]))
            .collect()
    } else {
        utils::points_from_area(&area, &category, &conn, last_seen)
            .await
            .map_err(actix_web::error::ErrorInternalServerError)?
    };
    println!(
        "[{}] Found Data Points: {}",
        mode.to_uppercase(),
        data_points.len()
    );

    stats.total_points = data_points.len();

    let mut clusters = clustering::main(
        data_points,
        fast,
        radius,
        min_points,
        only_unique,
        area,
        &mut stats,
    );

    if mode.eq("route") && !clusters.is_empty() {
        println!("Cluster Length: {}", clusters.len());
        println!("Routing for {} seconds...", routing_time);

        let tour = tsp::multi(&clusters, route_chunk_size, routing_time, fast);
        println!("Tour Length {}", tour.len());
        let mut final_clusters = VecDeque::<PointArray>::new();

        let mut rotate_count: usize = 0;

        let mut hash = HashSet::<usize>::new();

        for (i, index) in tour.into_iter().enumerate() {
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
        clusters = final_clusters.into();
    }

    let mut feature = clusters.to_feature(Some(&enum_type)).remove_last_coord();
    feature.add_instance_properties(Some(instance.to_string()), Some(&enum_type));

    let feature = feature.to_collection(Some(instance.clone()), None);

    if !instance.is_empty() && save_to_db {
        route::Query::upsert_from_collection(&conn.koji_db, feature.clone(), true, false)
            .await
            .map_err(actix_web::error::ErrorInternalServerError)?;
    }
    if save_to_scanner {
        if scanner_type == "rdm" {
            instance::Query::upsert_from_collection(&conn.data_db, feature.clone(), true).await
        } else if let Some(conn) = conn.unown_db.as_ref() {
            area::Query::upsert_from_collection(conn, feature.clone()).await
        } else {
            Err(DbErr::Custom(
                "Scanner not configured correctly".to_string(),
            ))
        }
        .map_err(actix_web::error::ErrorInternalServerError)?;

        request::update_project_api(&conn.koji_db, Some(scanner_type))
            .await
            .map_err(actix_web::error::ErrorInternalServerError)?;
    }

    Ok(utils::response::send(
        feature,
        return_type,
        Some(stats),
        benchmark_mode,
        Some(instance),
    ))
}
