use crate::utils::{request, response::Response};

use super::*;

use algorithms::{self, clustering, routing, stats::Stats};
use geo::{ChamberlainDuquetteArea, MultiPolygon, Polygon};

use geojson::Value;
use model::{
    api::{
        args::{Args, ArgsUnwrapped},
        sort_by::SortBy,
        FeatureHelpers, GeoFormats, ToCollection, ToFeature, ToSingleVec,
    },
    db::{area, geofence, instance, route, sea_orm_active_enums::Type},
    KojiDb, ScannerType,
};
use serde_json::json;

#[post("/bootstrap")]
async fn bootstrap(
    conn: web::Data<KojiDb>,
    payload: web::Json<Args>,
) -> Result<HttpResponse, Error> {
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
        sort_by,
        route_split_level,
        routing_args,
        ..
    } = payload.into_inner().init(Some("bootstrap"));

    if area.features.is_empty() && instance.is_empty() && parent.is_none() {
        return Ok(
            HttpResponse::BadRequest().json(Response::send_error("no_area_and_empty_instance"))
        );
    }

    let area = utils::create_or_find_collection(&instance, &conn, area, &parent, &vec![])
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;

    let mut stats = Stats::new(format!("Bootstrap | {:?}", calculation_mode), 1);

    let mut features: Vec<Feature> = algorithms::bootstrap::main(
        area,
        calculation_mode,
        radius,
        sort_by,
        s2_level,
        s2_size,
        route_split_level,
        &mut stats,
        &routing_args,
    );

    if parent.is_some() {
        let mut condensed = vec![];
        features
            .into_iter()
            .for_each(|feat| match feat.geometry.unwrap().value {
                geojson::Value::MultiPoint(points) => condensed.extend(points),
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

    let instance = if let Some(parent) = parent {
        let model = geofence::Query::get_one(&conn.koji, parent.to_string())
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
            if conn.scanner_type == ScannerType::Unown {
                "circle_pokemon"
            } else {
                "circle_smart_pokemon"
            },
        );
        if save_to_db {
            route::Query::upsert_from_geometry(&conn.koji, GeoFormats::Feature(feat.clone()))
                .await
                .map_err(actix_web::error::ErrorInternalServerError)?;
        }
        if save_to_scanner {
            if conn.scanner_type == ScannerType::Unown {
                area::Query::upsert_from_geometry(
                    &conn.controller,
                    GeoFormats::Feature(feat.clone()),
                )
                .await
            } else {
                instance::Query::upsert_from_geometry(
                    &conn.controller,
                    GeoFormats::Feature(feat.clone()),
                    true,
                )
                .await
            }
            .map_err(actix_web::error::ErrorInternalServerError)?;
        }
    }
    if save_to_scanner {
        request::update_project_api(&conn, Some(&conn.scanner_type))
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
    url: actix_web::web::Path<(String, String)>,
    payload: web::Json<Args>,
) -> Result<HttpResponse, Error> {
    let (mode, category) = url.into_inner();

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
        routing_args,
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
    let sort_by = if mode.eq("route") && sort_by == SortBy::Unset {
        SortBy::Custom(String::from("tsp"))
    } else {
        sort_by
    };

    let mut stats = Stats::new(
        format!("{:?} | {:?}", cluster_mode, calculation_mode),
        min_points,
    );
    let enum_type = if category == "gym" || category == "fort" {
        if conn.scanner_type == ScannerType::Unown {
            Type::CircleRaid
        } else {
            Type::CircleSmartRaid
        }
    } else if category == "pokestop" {
        Type::CircleQuest
    } else {
        if conn.scanner_type == ScannerType::Unown {
            Type::CirclePokemon
        } else {
            Type::CircleSmartPokemon
        }
    };

    let area = utils::create_or_find_collection(&instance, &conn, area, &parent, &data_points)
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;

    let data_points = if data_points.is_empty() {
        utils::points_from_area(&area, &category, &conn, last_seen, tth)
            .await
            .map_err(actix_web::error::ErrorInternalServerError)?
            .to_single_vec()
    } else {
        data_points
    };

    log::debug!(
        "[{}] Found Data Points: {}",
        mode.to_uppercase(),
        data_points.len()
    );

    let clusters = clustering::main(
        &data_points,
        cluster_mode,
        radius,
        min_points,
        &mut stats,
        cluster_split_level,
        max_clusters,
        calculation_mode,
        s2_level,
        s2_size,
        area,
    );
    let clusters = routing::main(
        &data_points,
        clusters,
        &sort_by,
        route_split_level,
        radius,
        &mut stats,
        &routing_args,
    );

    let mut feature = clusters
        .to_feature(Some(enum_type.clone()))
        .remove_last_coord();

    let instance = if let Some(parent) = parent {
        let model = geofence::Query::get_one(&conn.koji, parent.to_string())
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
            &conn.koji,
            GeoFormats::FeatureCollection(feature.clone()),
        )
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;
    }
    if save_to_scanner {
        if conn.scanner_type == ScannerType::Unown {
            area::Query::upsert_from_geometry(
                &conn.controller,
                GeoFormats::FeatureCollection(feature.clone()),
            )
            .await
        } else {
            instance::Query::upsert_from_geometry(
                &conn.controller,
                GeoFormats::FeatureCollection(feature.clone()),
                true,
            )
            .await
        }
        .map_err(actix_web::error::ErrorInternalServerError)?;

        request::update_project_api(&conn, Some(&conn.scanner_type))
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
        sort_by,
        radius,
        routing_args,
        ..
    } = payload.into_inner().init(Some("reroute"));
    let mut stats = Stats::new(String::from("Reroute"), 1);

    // For legacy compatibility
    let (clusters, data_points) = if clusters.is_empty() {
        (data_points, vec![])
    } else {
        (clusters, data_points)
    };
    stats.total_clusters = clusters.len();

    let clusters = routing::main(
        &data_points,
        clusters,
        &sort_by,
        route_split_level,
        radius,
        &mut stats,
        &routing_args,
    );

    let feature = clusters.to_feature(Some(mode.clone())).remove_last_coord();
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

    if clusters.is_empty() && data_points.is_empty() {
        return Ok(HttpResponse::BadRequest()
            .json(Response::send_error("no_clusters_or_data_points_found")));
    }
    let mut stats = Stats::new(format!("Route Stats | {:?}", mode), min_points);

    stats.distance_stats(&clusters);
    if !data_points.is_empty() {
        stats.cluster_stats(radius, &data_points, &clusters);
        stats.set_score();
    }

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
    let category = url.into_inner();

    let area = utils::create_or_find_collection(&instance, &conn, area, &parent, &data_points)
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

    if clusters.is_empty() && data_points.is_empty() {
        return Ok(HttpResponse::BadRequest()
            .json(Response::send_error("no_clusters_or_data_points_found")));
    }

    let mut stats = Stats::new(format!("Route Stats | {:?}", mode), min_points);

    stats.distance_stats(&clusters);
    if !data_points.is_empty() {
        stats.cluster_stats(radius, &data_points, &clusters);
        stats.set_score();
    }

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
