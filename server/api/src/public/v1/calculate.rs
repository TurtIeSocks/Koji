use super::*;

use std::{
    collections::{HashSet, VecDeque},
    time::Instant,
};

use algorithms::{bootstrapping, clustering, routing::tsp};
use geo::{Coord, HaversineDistance, Point};
use geojson::{Geometry, Value};

use model::{
    api::{
        args::{Args, ArgsUnwrapped, Response, Stats},
        point_array::PointArray,
        BBox, FeatureHelpers, ToCollection, ToFeature,
    },
    db::{
        area, geofence, gym, instance, pokestop, sea_orm_active_enums::Type, spawnpoint,
        GenericData,
    },
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
        ..
    } = payload.into_inner().init(Some("bootstrap"));

    if area.features.is_empty() && instance.is_empty() {
        return Ok(
            HttpResponse::BadRequest().json(Response::send_error("no_area_and_empty_instance"))
        );
    }

    let area = if area.features.is_empty() && !instance.is_empty() {
        if scanner_type.eq("rdm") {
            instance::Query::route(&conn.data_db, &instance).await
        } else {
            area::Query::route(&conn.unown_db.as_ref().unwrap(), &instance).await
        }
        .map_err(actix_web::error::ErrorInternalServerError)?
        .to_collection(None, None)
    } else {
        area
    };

    let mut stats = Stats::new();

    let time = Instant::now();

    let mut features: Vec<Feature> = area
        .into_iter()
        .map(|sub_area| bootstrapping::as_geojson(sub_area, radius, &mut stats))
        .collect();

    stats.cluster_time = time.elapsed().as_secs_f32();

    for feat in features.iter_mut() {
        if !feat.contains_property("name") && !instance.is_empty() {
            feat.set_property("name", instance.clone());
        }
        feat.set_property("type", Type::CirclePokemon.to_string());
        if save_to_db {
            area::Query::save(
                conn.unown_db.as_ref().unwrap(),
                feat.clone().to_collection(Some(instance.clone()), None),
            )
            .await
            .map_err(actix_web::error::ErrorInternalServerError)?;
        }
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
        Type::CircleRaid
    } else if category == "pokestop" {
        Type::ManualQuest
    } else {
        Type::CirclePokemon
    };

    let area = if !data_points.is_empty() {
        let bbox = BBox::new(
            &data_points
                .iter()
                .map(|p| Coord { x: p[1], y: p[0] })
                .collect(),
        );

        Ok(FeatureCollection {
            bbox: bbox.get_geojson_bbox(),
            features: vec![Feature {
                bbox: bbox.get_geojson_bbox(),
                geometry: Some(Geometry {
                    value: Value::Polygon(bbox.get_poly()),
                    bbox: None,
                    foreign_members: None,
                }),
                ..Default::default()
            }],
            foreign_members: None,
        })
    } else if !area.features.is_empty() {
        Ok(area)
    } else if !instance.is_empty() {
        let koji_area = geofence::Query::route(&conn.koji_db, &instance).await;
        let feature = match koji_area {
            Ok(area) => Ok(area),
            Err(_) => {
                if scanner_type.eq("rdm") {
                    instance::Query::route(&conn.data_db, &instance).await
                } else {
                    area::Query::route(&conn.unown_db.as_ref().unwrap(), &instance).await
                }
            }
        };
        match feature {
            Ok(feature) => Ok(feature.to_collection(None, None)),
            Err(err) => Err(err),
        }
    } else {
        Ok(<FeatureCollection as model::api::collection::Default>::default())
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
                gym::Query::area(&conn.data_db, &area, last_seen).await
            } else if category == "pokestop" {
                pokestop::Query::area(&conn.data_db, &area, last_seen).await
            } else {
                spawnpoint::Query::area(&conn.data_db, &area, last_seen).await
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
    feature.add_instance_properties(
        Some(instance.to_string()),
        if instance.eq("new_multipoint") {
            None
        } else {
            Some(&enum_type)
        },
    );

    feature.set_property("radius", radius);
    let feature = feature.to_collection(Some(instance.clone()), None);

    if !instance.is_empty() && save_to_db {
        area::Query::save(conn.unown_db.as_ref().unwrap(), feature.clone())
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
