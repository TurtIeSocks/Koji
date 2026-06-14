//! v2 typed CRUD for geofences.
//!
//! Geofences are geometry-bearing, so the `list`/`get_one` reads honor the
//! `?format=` (a.k.a. `rt`) return-type query via [`utils::response::send`]
//! exactly like v1 — the same `FeatureCollection` → SQL/text/array/poracle
//! catch-all. Writes go through the koji-db `Query` and are wrapped in
//! [`ApiResponse`](crate::utils::api_response::ApiResponse).
//!
//! Hand-written (not macro'd via [`super::resources`]) because the reads return
//! Koji-native geometry (`get_all_koji` / `descendants` / `get_one_koji`) for
//! [`utils::response::send`], unlike the plain-JSON resources.

use actix_web::{Error, HttpResponse, http::StatusCode, web};
use geojson::{Feature, Geometry};
use koji_core::ApiQueryArgs;
use koji_db::{
    KojiDb,
    db::geofence::{self, Anchor, HierarchySpec},
};

use crate::requests::{ReturnTypeArg, get_return_type};
use koji_dragonite::AreaMode;
use koji_events::EventDispatcher;
use serde_json::json;

use crate::dragonite::{GeofenceUpdated, TOPIC_GEOFENCE_UPDATED};
use crate::utils::{self, api_response::ApiResponse};

/// `GET /api/v2/geofences` — list all geofences as a `FeatureCollection`,
/// honoring `?format=`/`rt` (defaults to `featurecollection`).
///
/// With `?depth=N` or `?level=N` this becomes a recursive forest walk
/// (anchor = every `parent IS NULL` root): `depth=N` is the cumulative subtree
/// through level N, `level=N` is only the geofences exactly N levels down. The
/// two are mutually exclusive (both → 400). Omitted → the existing
/// non-recursive listing.
async fn list(
    conn: web::Data<KojiDb>,
    args: web::Query<ApiQueryArgs>,
) -> Result<HttpResponse, Error> {
    let args = args.into_inner();
    let return_type = get_return_type(
        args.rt
            .clone()
            .unwrap_or_else(|| "featurecollection".to_string()),
        &ReturnTypeArg::FeatureCollection,
    );

    let coll = match HierarchySpec::from_args(args.depth, args.level) {
        Err(_) => {
            return Ok(ApiResponse::fail(
                StatusCode::BAD_REQUEST,
                json!({ "hierarchy": "`depth` and `level` are mutually exclusive" }),
            ));
        }
        Ok(Some(spec)) => geofence::Query::descendants(&conn.koji, Anchor::Forest, spec)
            .await
            .map_err(actix_web::error::ErrorInternalServerError)?,
        Ok(None) => geofence::Query::get_all_koji(&conn.koji)
            .await
            .map_err(actix_web::error::ErrorInternalServerError)?,
    };

    Ok(utils::response::send(coll, return_type, None, false, None))
}

/// `POST /api/v2/geofences` — create a geofence → `201` ApiResponse.
async fn create(
    conn: web::Data<KojiDb>,
    payload: web::Json<serde_json::Value>,
) -> Result<HttpResponse, Error> {
    let record = geofence::Query::upsert_json_return(&conn.koji, 0, payload.into_inner())
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;
    Ok(ApiResponse::success_with_status(
        StatusCode::CREATED,
        record,
    ))
}

/// `GET /api/v2/geofences/{id}` — one geofence (by id or name) as a feature,
/// honoring `?format=`/`rt` (defaults to `feature`).
///
/// With `?depth=N` or `?level=N` this anchors a recursive subtree on the named
/// geofence: `depth=N` is the cumulative subtree through level N, `level=N` is
/// only the geofences exactly N levels down. The two are mutually exclusive
/// (both → 400). Omitted → the existing single-geofence response.
async fn get_one(
    conn: web::Data<KojiDb>,
    path: web::Path<String>,
    args: web::Query<ApiQueryArgs>,
) -> Result<HttpResponse, Error> {
    let id = path.into_inner();
    let args = args.into_inner();
    let return_type = get_return_type(
        args.rt.clone().unwrap_or_else(|| "feature".to_string()),
        &ReturnTypeArg::Feature,
    );

    let coll = match HierarchySpec::from_args(args.depth, args.level) {
        Err(_) => {
            return Ok(ApiResponse::fail(
                StatusCode::BAD_REQUEST,
                json!({ "hierarchy": "`depth` and `level` are mutually exclusive" }),
            ));
        }
        Ok(Some(spec)) => {
            let anchor = id
                .parse::<u32>()
                .map(Anchor::Id)
                .unwrap_or_else(|_| Anchor::Name(id));
            geofence::Query::descendants(&conn.koji, anchor, spec)
                .await
                .map_err(actix_web::error::ErrorInternalServerError)?
        }
        Ok(None) => {
            let geometry = geofence::Query::get_one_koji(&conn.koji, id)
                .await
                .map_err(actix_web::error::ErrorInternalServerError)?;
            koji_core::KojiGeometryCollection::new(vec![geometry])
        }
    };

    Ok(utils::response::send(coll, return_type, None, false, None))
}

/// `PATCH /api/v2/geofences/{id}` — update a geofence by id → ApiResponse.
async fn update(
    conn: web::Data<KojiDb>,
    path: web::Path<u32>,
    payload: web::Json<serde_json::Value>,
) -> Result<HttpResponse, Error> {
    let record =
        geofence::Query::upsert_json_return(&conn.koji, path.into_inner(), payload.into_inner())
            .await
            .map_err(actix_web::error::ErrorInternalServerError)?;
    Ok(ApiResponse::success(record))
}

/// `DELETE /api/v2/geofences/{id}` — delete a geofence → ApiResponse `{rows_affected}`.
async fn remove(conn: web::Data<KojiDb>, path: web::Path<u32>) -> Result<HttpResponse, Error> {
    let result = geofence::Query::delete(&conn.koji, path.into_inner())
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;
    Ok(ApiResponse::success(
        json!({ "rows_affected": result.rows_affected }),
    ))
}

/// `POST /api/v2/geofences/{id}/publish` — publish a geofence's fence to its
/// linked Dragonite area.
///
/// Replaces the v1 GET-that-mutates `/geofence/push/{id}` and the old direct
/// controller-DB write (P5, architecture §7): instead of writing the scanner DB
/// inline, it appends an [`area.geofence_updated`](TOPIC_GEOFENCE_UPDATED) event
/// to the outbox, which the dispatcher delivers to the `DragoniteSubscriber`
/// (PATCH `/v2/areas/{id}`). Returns `202 { event_id }`.
///
/// Gated on linkage: a geofence with no `dragonite_area_id` yields `422` (it is
/// not bound to a Dragonite area, so there is nothing to push to).
async fn publish(conn: web::Data<KojiDb>, path: web::Path<String>) -> Result<HttpResponse, Error> {
    let id = path.into_inner();

    // Resolve the geofence (by id or name) — 404 if it doesn't exist.
    let model = match geofence::Query::get_one(&conn.koji, id.clone()).await {
        Ok(model) => model,
        Err(_) => {
            return Ok(ApiResponse::fail(
                StatusCode::NOT_FOUND,
                json!({ "geofence": format!("no geofence {id}") }),
            ));
        }
    };

    // Linkage gate: only linked geofences can be pushed.
    let Some(dragonite_area_id) = model.dragonite_area_id else {
        return Ok(ApiResponse::fail(
            StatusCode::UNPROCESSABLE_ENTITY,
            json!({ "dragonite_area_id": "geofence is not linked to a Dragonite area" }),
        ));
    };

    // Carry the fence geometry as a GeoJSON Feature (Dragonite accepts a Feature
    // wrapping the Polygon/MultiPolygon). This builds a property-less,
    // id-less Feature straight off the stored geometry — deliberately NOT the
    // Phase 1 `Feature::from(&KojiGeometry)`, which would attach KojiMeta
    // (`id`/`name`/`mode`/...) as `properties`. `area_geofence_patch` ships the
    // whole `Feature` (`feature_to_geofence` clones it verbatim) to Dragonite, so
    // adding those props would change the external PATCH payload; the
    // `GeofenceUpdated.geofence` shape is locked, so it stays property-less. This
    // path already touches no `To*` matrix method, so it needs no rewire.
    let geometry = Geometry::from_json_value(model.geometry.clone())
        .map_err(actix_web::error::ErrorInternalServerError)?;
    let feature = Feature {
        bbox: None,
        geometry: Some(geometry),
        id: None,
        properties: None,
        foreign_members: None,
    };

    let payload = GeofenceUpdated {
        dragonite_area_id: dragonite_area_id as i64,
        mode: AreaMode::Base,
        geofence: feature,
    };
    let event_id = EventDispatcher::publish(&conn.koji, TOPIC_GEOFENCE_UPDATED, &payload)
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;

    Ok(ApiResponse::success_with_status(
        StatusCode::ACCEPTED,
        json!({
            "geofence": model.id,
            "event_id": event_id.to_string(),
            "topic": TOPIC_GEOFENCE_UPDATED,
            "dragonite_area_id": dragonite_area_id,
        }),
    ))
}

/// The `web::Scope` wiring the geofence handlers under `/geofences`, mounted into
/// `/api/v2` by [`crate::start`]. The `/{id}/publish` sub-resource is registered
/// before the `/{id}` catch-all so the more specific route matches first.
pub(crate) fn scope() -> actix_web::Scope {
    web::scope("/geofences")
        .service(
            web::resource("")
                .route(web::get().to(list))
                .route(web::post().to(create)),
        )
        .service(web::resource("/{id}/publish").route(web::post().to(publish)))
        .service(
            web::resource("/{id}")
                .route(web::get().to(get_one))
                .route(web::patch().to(update))
                .route(web::delete().to(remove)),
        )
}
