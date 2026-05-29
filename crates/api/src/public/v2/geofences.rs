//! v2 typed CRUD for geofences.
//!
//! Geofences are geometry-bearing, so the `list`/`get_one` reads honor the
//! `?format=` (a.k.a. `rt`) return-type query via [`utils::response::send`]
//! exactly like v1 — the same `FeatureCollection` → SQL/text/array/poracle
//! catch-all. Writes go through the koji-db `Query` and are wrapped in
//! [`JSend`](crate::utils::jsend::JSend).
//!
//! Hand-written (not macro'd via [`super::resources`]) because the koji-db
//! signature differs: `get_all_collection` / `get_one_feature` take an
//! [`ApiQueryArgs`], unlike the plain-JSON resources.

use actix_web::{http::StatusCode, web, Error, HttpResponse};
use koji_core::{ApiQueryArgs, FeatureCtx, ReturnTypeArg, ToCollection};
use koji_db::{db::geofence, KojiDb};
use model::api::args::get_return_type;
use serde_json::json;

use crate::utils::{self, jsend::JSend};

/// `GET /api/v2/geofences` — list all geofences as a `FeatureCollection`,
/// honoring `?format=`/`rt` (defaults to `featurecollection`).
async fn list(
    conn: web::Data<KojiDb>,
    args: web::Query<ApiQueryArgs>,
) -> Result<HttpResponse, Error> {
    let args = args.into_inner();
    let return_type = get_return_type(
        args.rt.clone().unwrap_or_else(|| "featurecollection".to_string()),
        &ReturnTypeArg::FeatureCollection,
    );

    let fc = geofence::Query::get_all_collection(&conn.koji, &args)
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;

    Ok(utils::response::send(fc, return_type, None, false, None))
}

/// `POST /api/v2/geofences` — create a geofence → `201` JSend.
async fn create(
    conn: web::Data<KojiDb>,
    payload: web::Json<serde_json::Value>,
) -> Result<HttpResponse, Error> {
    let record = geofence::Query::upsert_json_return(&conn.koji, 0, payload.into_inner())
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;
    Ok(JSend::success_with_status(StatusCode::CREATED, record))
}

/// `GET /api/v2/geofences/{id}` — one geofence (by id or name) as a feature,
/// honoring `?format=`/`rt` (defaults to `feature`).
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

    let feature = geofence::Query::get_one_feature(&conn.koji, id, &args)
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;

    Ok(utils::response::send(
        feature.to_collection(&FeatureCtx::default()),
        return_type,
        None,
        false,
        None,
    ))
}

/// `PATCH /api/v2/geofences/{id}` — update a geofence by id → JSend.
async fn update(
    conn: web::Data<KojiDb>,
    path: web::Path<u32>,
    payload: web::Json<serde_json::Value>,
) -> Result<HttpResponse, Error> {
    let record =
        geofence::Query::upsert_json_return(&conn.koji, path.into_inner(), payload.into_inner())
            .await
            .map_err(actix_web::error::ErrorInternalServerError)?;
    Ok(JSend::success(record))
}

/// `DELETE /api/v2/geofences/{id}` — delete a geofence → JSend `{rows_affected}`.
async fn remove(conn: web::Data<KojiDb>, path: web::Path<u32>) -> Result<HttpResponse, Error> {
    let result = geofence::Query::delete(&conn.koji, path.into_inner())
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;
    Ok(JSend::success(json!({ "rows_affected": result.rows_affected })))
}

/// `POST /api/v2/geofences/{id}/publish` — publish a geofence to the scanner.
///
/// Replaces the v1 GET-that-mutates `/geofence/push/{id}`. For now this just
/// acknowledges with `202 Accepted`; the actual push + event emission lands in
/// P5.
async fn publish(
    _conn: web::Data<KojiDb>,
    path: web::Path<String>,
) -> Result<HttpResponse, Error> {
    let id = path.into_inner();
    // TODO(P5): emit area.geofence_updated via events.publish (push to scanner +
    // outbox dispatch); P4 only acknowledges.
    Ok(JSend::success_with_status(
        StatusCode::ACCEPTED,
        json!({ "geofence": id, "status": "accepted" }),
    ))
}

/// The `web::Scope` wiring the geofence handlers under `/geofences`, mounted into
/// `/api/v2` by [`crate::start`]. The `/{id}/publish` sub-resource is registered
/// before the `/{id}` catch-all so the more specific route matches first.
pub fn scope() -> actix_web::Scope {
    web::scope("/geofences")
        .service(web::resource("").route(web::get().to(list)).route(web::post().to(create)))
        .service(web::resource("/{id}/publish").route(web::post().to(publish)))
        .service(
            web::resource("/{id}")
                .route(web::get().to(get_one))
                .route(web::patch().to(update))
                .route(web::delete().to(remove)),
        )
}
