use crate::utils::{request, response::Response};

use super::*;

use std::{collections::VecDeque, time::Instant};

use algorithms::{
    bootstrapping, clustering,
    routing::{basic, tsp},
    s2,
    stats::Stats,
};
use geo::{ChamberlainDuquetteArea, MultiPolygon, Polygon};

use geojson::Value;
use model::{
    api::{
        args::{Args, ArgsUnwrapped, CalculationMode, SortBy},
        point_array::PointArray,
        FeatureHelpers, GeoFormats, Precision, ToCollection, ToFeature, ToSingleVec,
    },
    db::{area, geofence, instance, route, sea_orm_active_enums::Type, GenericData},
    KojiDb,
};
use serde_json::json;

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
        calculation_mode,
        s2_level,
        s2_size,
        parent,
        ..
    } = payload.into_inner().init(Some("bootstrap"));

    if area.features.is_empty() && instance.is_empty() && parent.is_none() {
        return Ok(
            HttpResponse::BadRequest().json(Response::send_error("no_area_and_empty_instance"))
        );
    }

    let area =
        utils::create_or_find_collection(&instance, scanner_type, &conn, area, &parent, &vec![])
            .await
            .map_err(actix_web::error::ErrorInternalServerError)?;

    let mut stats = Stats::new(format!("Bootstrap | {:?}", calculation_mode));

    let time = Instant::now();

    let mut features: Vec<Feature> = area
        .into_iter()
        .map(|sub_area| match calculation_mode {
            CalculationMode::Radius => bootstrapping::as_geojson(sub_area, radius, &mut stats),
            CalculationMode::S2 => s2::bootstrap(&sub_area, s2_level, s2_size, &mut stats),
        })
        .collect();

    if parent.is_some() {
        let mut condensed = vec![];
        features
            .into_iter()
            .for_each(|feat| match feat.geometry.unwrap().value {
                geojson::Value::MultiPoint(mut points) => condensed.append(&mut points),
                _ => {}
            });
        features = vec![Feature {
            geometry: Some(geojson::Geometry {
                bbox: None,
                foreign_members: None,
                value: geojson::Value::MultiPoint(condensed),
            }),
            ..Default::default()
        }]
    }
    stats.cluster_time = time.elapsed().as_secs_f32() as Precision;

    let instance = if let Some(parent) = parent {
        let model = geofence::Query::get_one(&conn.koji_db, parent.to_string())
            .await
            .map_err(actix_web::error::ErrorInternalServerError)?;
        model.name
    } else {
        instance
    };
    for feat in features.iter_mut() {
        if !feat.contains_property("__name") && !instance.is_empty() {
            feat.set_property("__name", instance.clone());
        }
        feat.set_property(
            "__mode",
            if scanner_type == "rdm" {
                "circle_smart_pokemon"
            } else {
                "circle_pokemon"
            },
        );
        if save_to_db {
            route::Query::upsert_from_geometry(&conn.koji_db, GeoFormats::Feature(feat.clone()))
                .await
                .map_err(actix_web::error::ErrorInternalServerError)?;
        }
        if save_to_scanner {
            if scanner_type == "rdm" {
                instance::Query::upsert_from_geometry(
                    &conn.data_db,
                    GeoFormats::Feature(feat.clone()),
                    true,
                )
                .await
            } else if let Some(conn) = conn.unown_db.as_ref() {
                area::Query::upsert_from_geometry(conn, GeoFormats::Feature(feat.clone())).await
            } else {
                Err(DbErr::Custom(
                    "Scanner not configured correctly".to_string(),
                ))
            }
            .map_err(actix_web::error::ErrorInternalServerError)?;
        }
    }
    if save_to_scanner {
        request::update_project_api(&conn, Some(scanner_type))
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
        cluster_mode,
        cluster_split_level,
        data_points,
        instance,
        min_points,
        radius,
        return_type,
        save_to_db,
        save_to_scanner,
        last_seen,
        sort_by,
        tth,
        route_split_level,
        calculation_mode,
        s2_level,
        s2_size,
        parent,
        max_clusters,
        ..
    } = payload.into_inner().init(Some(&mode));

    if area.features.is_empty() && instance.is_empty() && data_points.is_empty() && parent.is_none()
    {
        return Ok(
            HttpResponse::BadRequest().json(Response::send_error("no_area_instance_data_points"))
        );
    }

    let mut stats = Stats::new(format!("{:?} | {:?}", cluster_mode, calculation_mode));
    let enum_type = if category == "gym" || category == "fort" {
        if scanner_type == "rdm" {
            Type::CircleSmartRaid
        } else {
            Type::CircleRaid
        }
    } else if category == "pokestop" {
        Type::CircleQuest
    } else {
        if scanner_type == "rdm" {
            Type::CircleSmartPokemon
        } else {
            Type::CirclePokemon
        }
    };

    let area = utils::create_or_find_collection(
        &instance,
        scanner_type,
        &conn,
        area,
        &parent,
        &data_points,
    )
    .await
    .map_err(actix_web::error::ErrorInternalServerError)?;

    let data_points = if !data_points.is_empty() {
        data_points
            .iter()
            .map(|p| GenericData::new("".to_string(), p[0], p[1]))
            .collect()
    } else {
        utils::points_from_area(&area, &category, &conn, last_seen, tth)
            .await
            .map_err(actix_web::error::ErrorInternalServerError)?
    }
    .to_single_vec();

    log::debug!(
        "[{}] Found Data Points: {}",
        mode.to_uppercase(),
        data_points.len()
    );

    let mut clusters = match calculation_mode {
        CalculationMode::Radius => clustering::main(
            &data_points,
            cluster_mode,
            radius,
            min_points,
            &mut stats,
            cluster_split_level,
            max_clusters,
        ),
        CalculationMode::S2 => area
            .into_iter()
            .flat_map(|feature| s2::cluster(feature, &data_points, s2_level, s2_size, &mut stats))
            .collect(),
    };

    let route_time = Instant::now();
    clusters = if (mode.eq("route") || sort_by == SortBy::TSP) && !clusters.is_empty() {
        log::info!("Cluster Length: {}", clusters.len());
        let tour = tsp::multi(&clusters, route_split_level);

        log::info!("Tour Length {}", tour.len());
        let mut final_clusters = VecDeque::<PointArray>::new();

        let mut rotate_count: usize = 0;

        for (i, [lat, lon]) in tour.into_iter().enumerate() {
            if stats.best_clusters.len() > 0
                && lat == stats.best_clusters[0][0]
                && lon == stats.best_clusters[0][1]
            {
                rotate_count = i;
                log::debug!("Found Best! {}, {} - {}", lat, lon, i);
            }
            final_clusters.push_back([lat, lon]);
        }
        final_clusters.rotate_left(rotate_count);

        final_clusters.into()
    } else {
        basic::sort(&data_points, clusters, radius, sort_by)
    };
    stats.set_route_time(route_time);
    stats.distance_stats(&clusters);

    let mut feature = clusters
        .to_feature(Some(enum_type.clone()))
        .remove_last_coord();

    let instance = if let Some(parent) = parent {
        let model = geofence::Query::get_one(&conn.koji_db, parent.to_string())
            .await
            .map_err(actix_web::error::ErrorInternalServerError)?;
        model.name
    } else {
        instance
    };
    feature.add_instance_properties(Some(instance.to_string()), Some(enum_type));
    let feature = feature.to_collection(Some(instance.clone()), None);

    if !instance.is_empty() && save_to_db {
        route::Query::upsert_from_geometry(
            &conn.koji_db,
            GeoFormats::FeatureCollection(feature.clone()),
        )
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;
    }
    if save_to_scanner {
        if scanner_type == "rdm" {
            instance::Query::upsert_from_geometry(
                &conn.data_db,
                GeoFormats::FeatureCollection(feature.clone()),
                true,
            )
            .await
        } else if let Some(conn) = conn.unown_db.as_ref() {
            area::Query::upsert_from_geometry(conn, GeoFormats::FeatureCollection(feature.clone()))
                .await
        } else {
            Err(DbErr::Custom(
                "Scanner not configured correctly".to_string(),
            ))
        }
        .map_err(actix_web::error::ErrorInternalServerError)?;

        request::update_project_api(&conn, Some(scanner_type))
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

#[post("/reroute")]
async fn reroute(payload: web::Json<Args>) -> Result<HttpResponse, Error> {
    let ArgsUnwrapped {
        benchmark_mode,
        data_points,
        clusters,
        return_type,
        route_split_level,
        instance,
        mode,
        ..
    } = payload.into_inner().init(Some("reroute"));
    let mut stats = Stats::new(String::from("Reroute"));

    // For legacy compatibility
    let data_points = if clusters.is_empty() {
        data_points
    } else {
        clusters
    };

    stats.total_clusters = data_points.len();

    let final_clusters = tsp::multi(&data_points, route_split_level);
    log::info!("Tour Length {}", final_clusters.len());

    stats.distance_stats(&final_clusters);

    let feature = final_clusters
        .to_feature(Some(mode.clone()))
        .remove_last_coord();
    let feature = feature.to_collection(Some(instance.clone()), Some(mode));

    Ok(utils::response::send(
        feature,
        return_type,
        Some(stats),
        benchmark_mode,
        Some(instance),
    ))
}

#[post("/route-stats")]
async fn route_stats(payload: web::Json<Args>) -> Result<HttpResponse, Error> {
    let ArgsUnwrapped {
        clusters,
        data_points,
        instance,
        radius,
        mode,
        min_points,
        ..
    } = payload.into_inner().init(Some("route-stats"));

    if clusters.is_empty() || data_points.is_empty() {
        return Ok(HttpResponse::BadRequest()
            .json(Response::send_error("no_clusters_or_data_points_found")));
    }
    let mut stats = Stats::new(format!("Route Stats | {:?}", mode));
    stats.cluster_stats(radius, &data_points, &clusters);
    stats.distance_stats(&clusters);
    stats.set_score(min_points);

    let feature = clusters.to_feature(Some(mode.clone())).remove_last_coord();
    let feature = feature.to_collection(Some(instance.clone()), Some(mode));

    Ok(utils::response::send(
        feature,
        model::api::args::ReturnTypeArg::Feature,
        Some(stats),
        true,
        Some(instance),
    ))
}

#[post("/route-stats/{category}")]
async fn route_stats_category(
    scanner_type: web::Data<String>,
    conn: web::Data<KojiDb>,
    url: actix_web::web::Path<String>,
    payload: web::Json<Args>,
) -> Result<HttpResponse, Error> {
    let ArgsUnwrapped {
        clusters,
        data_points,
        instance,
        radius,
        mode,
        area,
        parent,
        last_seen,
        tth,
        min_points,
        ..
    } = payload.into_inner().init(Some("route-stats"));
    let scanner_type = scanner_type.as_ref();
    let category = url.into_inner();

    let area = utils::create_or_find_collection(
        &instance,
        scanner_type,
        &conn,
        area,
        &parent,
        &data_points,
    )
    .await
    .map_err(actix_web::error::ErrorInternalServerError)?;

    let data_points = if !data_points.is_empty() {
        data_points
    } else {
        utils::points_from_area(&area, &category, &conn, last_seen, tth)
            .await
            .map_err(actix_web::error::ErrorInternalServerError)?
            .to_single_vec()
    };

    if clusters.is_empty() || data_points.is_empty() {
        return Ok(HttpResponse::BadRequest()
            .json(Response::send_error("no_clusters_or_data_points_found")));
    }

    let mut stats = Stats::new(format!("Route Stats | {:?}", mode));

    stats.cluster_stats(radius, &data_points, &clusters);
    stats.distance_stats(&clusters);

    stats.set_score(min_points);
    let feature = clusters.to_feature(Some(mode.clone())).remove_last_coord();
    let feature = feature.to_collection(Some(instance.clone()), Some(mode));

    Ok(utils::response::send(
        feature,
        model::api::args::ReturnTypeArg::Feature,
        Some(stats),
        true,
        Some(instance),
    ))
}

#[post("/area")]
async fn calculate_area(payload: web::Json<Args>) -> Result<HttpResponse, Error> {
    let ArgsUnwrapped { area, .. } = payload.into_inner().init(Some("calculate_area"));

    let mut total_area = 0.;

    for feature in area.into_iter() {
        if let Some(geometry) = feature.geometry {
            match geometry.value {
                Value::MultiPolygon(_) => match MultiPolygon::<f64>::try_from(&geometry) {
                    Ok(mp) => {
                        total_area += mp.chamberlain_duquette_unsigned_area();
                    }
                    Err(err) => log::error!("Unable to calculate area for MultiPolygon: {}", err),
                },
                Value::Polygon(_) => match Polygon::<f64>::try_from(&geometry) {
                    Ok(poly) => {
                        total_area += poly.chamberlain_duquette_unsigned_area();
                    }
                    Err(err) => log::error!("Unable to calculate area for Polygon: {}", err),
                },
                _ => {}
            }
        }
    }

    log::info!("[AREA] Found total area: {}", total_area);

    Ok(HttpResponse::Ok().json(Response {
        data: Some(json!({ "area": total_area })),
        message: "Success".to_string(),
        status: "ok".to_string(),
        stats: None,
        status_code: 200,
    }))
}
