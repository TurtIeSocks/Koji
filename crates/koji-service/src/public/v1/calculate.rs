use crate::utils::response::Response;

use super::*;

use algorithms::{self, clustering, routing, stats::Stats};
use geo::{ChamberlainDuquetteArea, MultiPolygon, Polygon};

use geojson::Value;
use koji_core::{
    BootstrapConfig, ClusteringConfig, FeatureCtx, FeatureHelpers, GeoFormats, RoutingConfig,
    S2Config, SortBy, ToCollection, ToFeature,
};
use koji_db::{
    KojiDb,
    db::{geofence, route, sea_orm_active_enums::Type},
};
use koji_scanner::GenericDataToVec;
use model::api::args::{Args, ArgsUnwrapped};
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
        calculation_mode,
        s2_level,
        s2_size,
        parent,
        sort_by,
        route_split_level,
        routing_args,
        bootstrapping_args,
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
        &BootstrapConfig {
            calculation_mode,
            radius,
            s2: S2Config {
                level: s2_level,
                size: s2_size,
            },
            plugin_args: bootstrapping_args,
        },
        &RoutingConfig {
            sort_by,
            route_split_level,
            plugin_args: routing_args,
        },
        &mut stats,
    );

    if parent.is_some() {
        let mut condensed = vec![];
        features.into_iter().for_each(|feat| {
            if let geojson::Value::MultiPoint(points) = feat.geometry.unwrap().value {
                condensed.extend(points)
            }
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
        if !feat.contains_property("__mode") {
            feat.set_property("__mode", "circle_pokemon");
        }
        if save_to_db {
            route::Query::upsert_from_geometry(&conn.koji, GeoFormats::Feature(feat.clone()))
                .await
                .map_err(actix_web::error::ErrorInternalServerError)?;
        }
    }

    Ok(utils::response::send(
        features.to_collection(&FeatureCtx::new().with_name(instance.clone())),
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
        clustering_args,
        center_clusters,
        genetic_post_processing,
        dev,
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
        Type::CircleRaid
    } else if category == "station" {
        Type::CircleStation
    } else if category == "pokestop" {
        Type::CircleQuest
    } else {
        Type::CirclePokemon
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
        &ClusteringConfig {
            mode: cluster_mode,
            radius,
            min_points,
            max_clusters,
            cluster_split_level,
            calculation_mode,
            s2: S2Config {
                level: s2_level,
                size: s2_size,
            },
            center_clusters,
            genetic_post_processing,
            plugin_args: clustering_args,
        },
        area,
        dev.bypass_adaptive_partition,
        &mut stats,
    );
    let clusters = routing::main(
        &data_points,
        clusters,
        radius,
        &RoutingConfig {
            sort_by,
            route_split_level,
            plugin_args: routing_args,
        },
        &mut stats,
    );

    let mut feature = clusters
        .to_feature(&FeatureCtx::new().with_type(enum_type.clone().into()))
        .remove_last_coord();

    let instance = if let Some(parent) = parent {
        let model = geofence::Query::get_one(&conn.koji, parent.to_string())
            .await
            .map_err(actix_web::error::ErrorInternalServerError)?;
        model.name
    } else {
        instance
    };
    feature.add_instance_properties(&FeatureCtx {
        name: Some(instance.to_string()),
        fence_type: Some(enum_type.into()),
    });
    let feature = feature.to_collection(&FeatureCtx::new().with_name(instance.clone()));

    if !instance.is_empty() && save_to_db {
        route::Query::upsert_from_geometry(
            &conn.koji,
            GeoFormats::FeatureCollection(feature.clone()),
        )
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
        radius,
        &RoutingConfig {
            sort_by,
            route_split_level,
            plugin_args: routing_args,
        },
        &mut stats,
    );

    let feature = clusters
        .to_feature(&FeatureCtx::new().with_type(mode))
        .remove_last_coord();
    let feature = feature.to_collection(
        &FeatureCtx::new()
            .with_name(instance.clone())
            .with_type(mode),
    );

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

    let feature = clusters
        .to_feature(&FeatureCtx::new().with_type(mode))
        .remove_last_coord();
    let feature = feature.to_collection(
        &FeatureCtx::new()
            .with_name(instance.clone())
            .with_type(mode),
    );

    Ok(utils::response::send(
        feature,
        koji_core::ReturnTypeArg::Feature,
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

    let feature = clusters
        .to_feature(&FeatureCtx::new().with_type(mode))
        .remove_last_coord();
    let feature = feature.to_collection(
        &FeatureCtx::new()
            .with_name(instance.clone())
            .with_type(mode),
    );

    Ok(utils::response::send(
        feature,
        koji_core::ReturnTypeArg::Feature,
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
