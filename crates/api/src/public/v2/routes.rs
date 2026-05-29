//! v2 typed CRUD for routes.
//!
//! Routes are geometry-bearing, so the `list`/`get_one` reads honor the
//! `?format=`/`rt` return-type query via [`utils::response::send`] (the same
//! catch-all v1 uses) and the `?internal=` flag. Writes go through the koji-db
//! `Query` and are wrapped in [`JSend`](crate::utils::jsend::JSend).
//!
//! Hand-written (not macro'd via [`super::resources`]) because the koji-db
//! signature differs: `as_collection` / `get_one_feature` take an `internal`
//! bool rather than the [`ApiQueryArgs`] the plain-JSON resources don't need at
//! all.

use actix_web::{http::StatusCode, web, Error, HttpResponse};
use koji_core::{ApiQueryArgs, FeatureCtx, ReturnTypeArg, ToCollection};
use koji_db::{db::route, KojiDb};
use model::api::args::get_return_type;
use serde_json::json;

use crate::utils::{self, jsend::JSend};

/// `GET /api/v2/routes` — list all routes as a `FeatureCollection`, honoring
/// `?format=`/`rt` (defaults to `featurecollection`) and `?internal=`.
async fn list(
    conn: web::Data<KojiDb>,
    args: web::Query<ApiQueryArgs>,
) -> Result<HttpResponse, Error> {
    let args = args.into_inner();
    let return_type = get_return_type(
        args.rt.clone().unwrap_or_else(|| "featurecollection".to_string()),
        &ReturnTypeArg::FeatureCollection,
    );

    let fc = route::Query::as_collection(&conn.koji, args.internal.unwrap_or(false))
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;

    Ok(utils::response::send(fc, return_type, None, false, None))
}

/// `POST /api/v2/routes` — create a route → `201` JSend.
async fn create(
    conn: web::Data<KojiDb>,
    payload: web::Json<serde_json::Value>,
) -> Result<HttpResponse, Error> {
    let record = route::Query::upsert_json_return(&conn.koji, 0, payload.into_inner())
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;
    Ok(JSend::success_with_status(StatusCode::CREATED, record))
}

/// `GET /api/v2/routes/{id}` — one route (by id or name) as a feature, honoring
/// `?format=`/`rt` (defaults to `feature`) and `?internal=`.
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

    let feature = route::Query::get_one_feature(&conn.koji, id, args.internal.unwrap_or(false))
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

/// `PATCH /api/v2/routes/{id}` — update a route by id → JSend.
async fn update(
    conn: web::Data<KojiDb>,
    path: web::Path<u32>,
    payload: web::Json<serde_json::Value>,
) -> Result<HttpResponse, Error> {
    let record =
        route::Query::upsert_json_return(&conn.koji, path.into_inner(), payload.into_inner())
            .await
            .map_err(actix_web::error::ErrorInternalServerError)?;
    Ok(JSend::success(record))
}

/// `DELETE /api/v2/routes/{id}` — delete a route → JSend `{rows_affected}`.
async fn remove(conn: web::Data<KojiDb>, path: web::Path<u32>) -> Result<HttpResponse, Error> {
    let result = route::Query::delete(&conn.koji, path.into_inner())
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;
    Ok(JSend::success(json!({ "rows_affected": result.rows_affected })))
}

/// The `web::Scope` wiring the route handlers under `/routes`, mounted into
/// `/api/v2` by [`crate::start`].
pub fn scope() -> actix_web::Scope {
    web::scope("/routes")
        .service(web::resource("").route(web::get().to(list)).route(web::post().to(create)))
        .service(
            web::resource("/{id}")
                .route(web::get().to(get_one))
                .route(web::patch().to(update))
                .route(web::delete().to(remove)),
        )
}
