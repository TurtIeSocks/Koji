//! v1 calc shim — now **queue-backed**.
//!
//! Each calc endpoint resolves its async inputs (area / scanner data points /
//! parent-name), builds a [`CalcPayload`], runs it through the **same job queue +
//! sync bridge** the v2 surface uses ([`run_via_queue`]), then re-wraps the
//! result in the legacy `{message,status,status_code,data,stats}` envelope via
//! [`utils::response::send`]. This removes the last inline-synchronous algorithm
//! execution path (architecture pain #1) — v1 calc now coalesces + runs on the
//! worker pool exactly like v2.
//!
//! `/area` is the lone exception: it is a pure geometry area sum (no clustering /
//! routing / scanner / DB), so it stays inline — routing it through the queue
//! would be pointless.

use std::time::Duration;

use crate::utils::response::Response;

use super::*;

use algorithms::stats::Stats;
use geo::{ChamberlainDuquetteArea, MultiPolygon, Polygon};
use geojson::{FeatureCollection, Geometry, Value};
use koji_core::{GeoFormats, ReturnTypeArg};
use koji_db::{
    KojiDb,
    db::{geofence, route},
};
use koji_jobs::{AwaitError, JobOutcome, JobQueue, dedup_key};
use model::api::args::{Args, ArgsUnwrapped};
use serde_json::{Value as JsonValue, json};

use crate::public::v2::calc::{CALC_KIND, CalcPayload};

/// Sync calc out-prioritizes batch enqueues (mirrors the v2 sync bridge).
const PRIORITY_HIGH: i16 = 100;
/// Block up to just under the typical 300s proxy timeout for the result.
const SYNC_WAIT: Duration = Duration::from_secs(290);

/// Parse the raw request body into [`Args`], surfacing a 400 on malformed JSON.
fn parse_args(request_json: &JsonValue) -> Result<Args, HttpResponse> {
    serde_json::from_value(request_json.clone()).map_err(|e| {
        HttpResponse::BadRequest().json(Response::send_error(&format!("invalid_request: {e}")))
    })
}

/// Enqueue a calc job (HIGH, content-deduped) and block for its result via the
/// sync bridge, returning the computed `FeatureCollection` + `Stats`. A
/// benchmark-mode job carries no geojson, so an empty collection is returned
/// (the caller passes `benchmark = true` to `send`, which ignores it).
async fn run_via_queue(
    jobs: &JobQueue,
    payload: CalcPayload,
) -> Result<(FeatureCollection, Stats), Error> {
    let key = dedup_key(
        CALC_KIND,
        &serde_json::to_value(&payload).map_err(actix_web::error::ErrorInternalServerError)?,
    );
    let id = jobs
        .enqueue_or_attach(CALC_KIND, Some(&key), &payload, PRIORITY_HIGH)
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;

    match jobs.await_result(id, SYNC_WAIT).await {
        Ok(JobOutcome::Succeeded(result)) => {
            let stats: Stats =
                serde_json::from_value(result.get("stats").cloned().unwrap_or(JsonValue::Null))
                    .map_err(actix_web::error::ErrorInternalServerError)?;
            let data = result.get("data").cloned().unwrap_or(JsonValue::Null);
            let collection = if data.is_null() {
                FeatureCollection::default()
            } else {
                serde_json::from_value(data).map_err(actix_web::error::ErrorInternalServerError)?
            };
            Ok((collection, stats))
        }
        Ok(JobOutcome::Failed { error, .. }) => {
            Err(actix_web::error::ErrorInternalServerError(error))
        }
        Ok(JobOutcome::Canceled) => Err(actix_web::error::ErrorInternalServerError(
            "calc job was canceled",
        )),
        Err(AwaitError::Timeout) => Err(actix_web::error::ErrorGatewayTimeout(
            "calculation still running; retry",
        )),
        Err(e) => Err(actix_web::error::ErrorInternalServerError(e.to_string())),
    }
}

/// Resolve the instance display name: a `parent` id resolves to that geofence's
/// name (legacy behavior); otherwise the supplied instance.
async fn resolve_instance(
    conn: &KojiDb,
    instance: String,
    parent: &Option<koji_core::UnknownId>,
) -> Result<String, Error> {
    if let Some(parent) = parent {
        let model = geofence::Query::get_one(&conn.koji, parent.to_string())
            .await
            .map_err(actix_web::error::ErrorInternalServerError)?;
        Ok(model.name)
    } else {
        Ok(instance)
    }
}

#[post("/bootstrap")]
async fn bootstrap(
    conn: web::Data<KojiDb>,
    jobs: web::Data<JobQueue>,
    body: web::Json<JsonValue>,
) -> Result<HttpResponse, Error> {
    let mut request_json = body.into_inner();
    let args = match parse_args(&request_json) {
        Ok(args) => args,
        Err(resp) => return Ok(resp),
    };
    let ArgsUnwrapped {
        area,
        benchmark_mode,
        instance,
        return_type,
        save_to_db,
        parent,
        ..
    } = args.init(Some("bootstrap"));

    if area.features.is_empty() && instance.is_empty() && parent.is_none() {
        return Ok(
            HttpResponse::BadRequest().json(Response::send_error("no_area_and_empty_instance"))
        );
    }

    let area = utils::create_or_find_collection(&instance, &conn, area, &parent, &vec![])
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;
    let instance = resolve_instance(&conn, instance, &parent).await?;

    // Inject the resolved instance so the worker labels features with it.
    request_json["instance"] = JsonValue::from(instance.clone());

    let (mut collection, stats) = run_via_queue(
        &jobs,
        CalcPayload {
            mode: "bootstrap".to_string(),
            category: String::new(),
            request: request_json,
            area,
            data_points: vec![],
            clusters: vec![],
        },
    )
    .await?;

    // Parent requests condense every bootstrap feature's points into one
    // MultiPoint feature (legacy behavior).
    if parent.is_some() {
        let mut condensed = vec![];
        for feat in collection.features.drain(..) {
            if let Some(Value::MultiPoint(points)) = feat.geometry.map(|g| g.value) {
                condensed.extend(points);
            }
        }
        collection.features = vec![Feature {
            geometry: Some(Geometry {
                bbox: None,
                foreign_members: None,
                value: Value::MultiPoint(condensed),
            }),
            ..Default::default()
        }];
    }

    // Label + (optionally) persist to the Koji route table.
    for feat in collection.features.iter_mut() {
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

    let coll = koji_core::KojiGeometryCollection::try_from(collection)
        .map_err(actix_web::error::ErrorInternalServerError)?;
    Ok(utils::response::send(
        coll,
        return_type,
        Some(stats),
        benchmark_mode,
        Some(instance),
    ))
}

#[post("/{mode}/{category}")]
async fn cluster(
    conn: web::Data<KojiDb>,
    jobs: web::Data<JobQueue>,
    url: actix_web::web::Path<(String, String)>,
    body: web::Json<JsonValue>,
) -> Result<HttpResponse, Error> {
    let (mode, category) = url.into_inner();
    let mut request_json = body.into_inner();
    let args = match parse_args(&request_json) {
        Ok(args) => args,
        Err(resp) => return Ok(resp),
    };
    let ArgsUnwrapped {
        area,
        benchmark_mode,
        data_points,
        instance,
        return_type,
        save_to_db,
        last_seen,
        tth,
        parent,
        ..
    } = args.init(Some(&mode));

    if area.features.is_empty() && instance.is_empty() && data_points.is_empty() && parent.is_none()
    {
        return Ok(
            HttpResponse::BadRequest().json(Response::send_error("no_area_instance_data_points"))
        );
    }

    let area = utils::create_or_find_collection(&instance, &conn, area, &parent, &data_points)
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;

    let data_points = if data_points.is_empty() {
        use koji_scanner::GenericDataToVec;
        utils::points_from_area(&area, &category, &conn, last_seen, tth)
            .await
            .map_err(actix_web::error::ErrorInternalServerError)?
            .to_single_vec()
    } else {
        data_points
    };
    let instance = resolve_instance(&conn, instance, &parent).await?;
    request_json["instance"] = JsonValue::from(instance.clone());

    let (collection, stats) = run_via_queue(
        &jobs,
        CalcPayload {
            mode,
            category,
            request: request_json,
            area,
            data_points,
            clusters: vec![],
        },
    )
    .await?;

    if !instance.is_empty() && save_to_db {
        route::Query::upsert_from_geometry(
            &conn.koji,
            GeoFormats::FeatureCollection(collection.clone()),
        )
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;
    }

    let coll = koji_core::KojiGeometryCollection::try_from(collection)
        .map_err(actix_web::error::ErrorInternalServerError)?;
    Ok(utils::response::send(
        coll,
        return_type,
        Some(stats),
        benchmark_mode,
        Some(instance),
    ))
}

#[post("/reroute")]
async fn reroute(
    jobs: web::Data<JobQueue>,
    body: web::Json<JsonValue>,
) -> Result<HttpResponse, Error> {
    let request_json = body.into_inner();
    let args = match parse_args(&request_json) {
        Ok(args) => args,
        Err(resp) => return Ok(resp),
    };
    let ArgsUnwrapped {
        benchmark_mode,
        return_type,
        instance,
        clusters,
        data_points,
        ..
    } = args.init(Some("reroute"));

    // The worker applies the legacy clusters/data_points swap in `run_reroute`.
    let (collection, stats) = run_via_queue(
        &jobs,
        CalcPayload {
            mode: "reroute".to_string(),
            category: String::new(),
            request: request_json,
            area: FeatureCollection::default(),
            data_points,
            clusters,
        },
    )
    .await?;

    let coll = koji_core::KojiGeometryCollection::try_from(collection)
        .map_err(actix_web::error::ErrorInternalServerError)?;
    Ok(utils::response::send(
        coll,
        return_type,
        Some(stats),
        benchmark_mode,
        Some(instance),
    ))
}

#[post("/route-stats")]
async fn route_stats(
    jobs: web::Data<JobQueue>,
    body: web::Json<JsonValue>,
) -> Result<HttpResponse, Error> {
    let request_json = body.into_inner();
    let args = match parse_args(&request_json) {
        Ok(args) => args,
        Err(resp) => return Ok(resp),
    };
    let ArgsUnwrapped {
        clusters,
        data_points,
        instance,
        ..
    } = args.init(Some("route-stats"));

    if clusters.is_empty() && data_points.is_empty() {
        return Ok(HttpResponse::BadRequest()
            .json(Response::send_error("no_clusters_or_data_points_found")));
    }

    let (collection, stats) = run_via_queue(
        &jobs,
        CalcPayload {
            mode: "route-stats".to_string(),
            category: String::new(),
            request: request_json,
            area: FeatureCollection::default(),
            data_points,
            clusters,
        },
    )
    .await?;

    let coll = koji_core::KojiGeometryCollection::try_from(collection)
        .map_err(actix_web::error::ErrorInternalServerError)?;
    Ok(utils::response::send(
        coll,
        ReturnTypeArg::Feature,
        Some(stats),
        true,
        Some(instance),
    ))
}

#[post("/route-stats/{category}")]
async fn route_stats_category(
    conn: web::Data<KojiDb>,
    jobs: web::Data<JobQueue>,
    url: actix_web::web::Path<String>,
    body: web::Json<JsonValue>,
) -> Result<HttpResponse, Error> {
    let request_json = body.into_inner();
    let args = match parse_args(&request_json) {
        Ok(args) => args,
        Err(resp) => return Ok(resp),
    };
    let category = url.into_inner();
    let ArgsUnwrapped {
        clusters,
        data_points,
        instance,
        area,
        parent,
        last_seen,
        tth,
        ..
    } = args.init(Some("route-stats"));

    let area = utils::create_or_find_collection(&instance, &conn, area, &parent, &data_points)
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;

    let data_points = if !data_points.is_empty() {
        data_points
    } else {
        use koji_scanner::GenericDataToVec;
        utils::points_from_area(&area, &category, &conn, last_seen, tth)
            .await
            .map_err(actix_web::error::ErrorInternalServerError)?
            .to_single_vec()
    };

    if clusters.is_empty() && data_points.is_empty() {
        return Ok(HttpResponse::BadRequest()
            .json(Response::send_error("no_clusters_or_data_points_found")));
    }

    let (collection, stats) = run_via_queue(
        &jobs,
        CalcPayload {
            mode: "route-stats".to_string(),
            category,
            request: request_json,
            area,
            data_points,
            clusters,
        },
    )
    .await?;

    let coll = koji_core::KojiGeometryCollection::try_from(collection)
        .map_err(actix_web::error::ErrorInternalServerError)?;
    Ok(utils::response::send(
        coll,
        ReturnTypeArg::Feature,
        Some(stats),
        true,
        Some(instance),
    ))
}

/// Pure geometry area sum — no clustering / routing / scanner / DB, so it stays
/// inline (the queue would add nothing).
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
