//! v2 typed CRUD for routes.
//!
//! Routes are geometry-bearing, so the `list`/`get_one` reads honor the
//! `?format=`/`rt` return-type query via [`utils::response::send`] (the same
//! catch-all v1 uses) and the `?internal=` flag. Writes go through the koji-db
//! `Query` and are wrapped in [`ApiResponse`](crate::utils::api_response::ApiResponse).
//!
//! Hand-written (not macro'd via [`super::resources`]) because the reads return
//! Koji-native geometry (`as_koji_collection` / `get_one_koji`) for
//! [`utils::response::send`] rather than the plain JSON the macro'd resources
//! emit.

use actix_web::{Error, HttpResponse, http::StatusCode, web};
use koji_core::{ApiQueryArgs, ReturnTypeArg};
use koji_db::{
    KojiDb,
    db::{geofence, route, sea_orm_active_enums::Mode},
};
use koji_dragonite::AreaMode;
use koji_events::EventDispatcher;
use model::api::args::get_return_type;
use serde_json::json;

use crate::dragonite::{RouteUpdated, TOPIC_ROUTE_UPDATED};
use crate::utils::{self, api_response::ApiResponse};

/// `GET /api/v2/routes` — list all routes as a `FeatureCollection`, honoring
/// `?format=`/`rt` (defaults to `featurecollection`) and `?internal=`.
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

    let coll = route::Query::as_koji_collection(&conn.koji)
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;

    Ok(utils::response::send(coll, return_type, None, false, None))
}

/// `POST /api/v2/routes` — create a route → `201` ApiResponse.
async fn create(
    conn: web::Data<KojiDb>,
    payload: web::Json<serde_json::Value>,
) -> Result<HttpResponse, Error> {
    let record = route::Query::upsert_json_return(&conn.koji, 0, payload.into_inner())
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;
    Ok(ApiResponse::success_with_status(
        StatusCode::CREATED,
        record,
    ))
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

    let geometry = route::Query::get_one_koji(&conn.koji, id)
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;

    Ok(utils::response::send(
        koji_core::KojiGeometryCollection::new(vec![geometry]),
        return_type,
        None,
        false,
        None,
    ))
}

/// `PATCH /api/v2/routes/{id}` — update a route by id → ApiResponse.
async fn update(
    conn: web::Data<KojiDb>,
    path: web::Path<u32>,
    payload: web::Json<serde_json::Value>,
) -> Result<HttpResponse, Error> {
    let record =
        route::Query::upsert_json_return(&conn.koji, path.into_inner(), payload.into_inner())
            .await
            .map_err(actix_web::error::ErrorInternalServerError)?;
    Ok(ApiResponse::success(record))
}

/// `DELETE /api/v2/routes/{id}` — delete a route → ApiResponse `{rows_affected}`.
async fn remove(conn: web::Data<KojiDb>, path: web::Path<u32>) -> Result<HttpResponse, Error> {
    let result = route::Query::delete(&conn.koji, path.into_inner())
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;
    Ok(ApiResponse::success(
        json!({ "rows_affected": result.rows_affected }),
    ))
}

/// Map a Koji route/geofence [`Mode`] to the Dragonite [`AreaMode`] its route
/// feeds. The 12→4 collapse already folded the historical variants, so this is
/// now a direct 1:1 mapping (`unset` → `Base`).
fn area_mode_for(mode: &Mode) -> AreaMode {
    match mode {
        Mode::Quest => AreaMode::Quest,
        Mode::Pokemon => AreaMode::Pokemon,
        Mode::Fort => AreaMode::Fort,
        Mode::Unset => AreaMode::Base,
    }
}

/// `POST /api/v2/routes/{id}/publish` — push a stored route to its linked
/// Dragonite area (the `area.route_updated` producer, symmetric with the
/// geofence publish). Resolves the route → its geofence → `dragonite_area_id`,
/// maps the route mode to an [`AreaMode`], and emits the event for the
/// `DragoniteSubscriber` to PATCH `/v2/areas/{id}`. Gated on linkage (an
/// unlinked geofence → `422`).
async fn publish(conn: web::Data<KojiDb>, path: web::Path<String>) -> Result<HttpResponse, Error> {
    let id = path.into_inner();

    let model = match route::Query::get_one(&conn.koji, id.clone()).await {
        Ok(model) => model,
        Err(_) => {
            return Ok(ApiResponse::fail(
                StatusCode::NOT_FOUND,
                json!({ "route": format!("no route {id}") }),
            ));
        }
    };

    // Linkage flows through the route's geofence.
    let fence = geofence::Query::get_one(&conn.koji, model.geofence_id.to_string())
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;
    let Some(dragonite_area_id) = fence.dragonite_area_id else {
        return Ok(ApiResponse::fail(
            StatusCode::UNPROCESSABLE_ENTITY,
            json!({ "dragonite_area_id": "route's geofence is not linked to a Dragonite area" }),
        ));
    };

    // Route points as a Koji `SingleVec` (`[lat, lon]`). Fetch the route as a
    // `KojiGeometry`, wrap it in a one-item collection, and flatten via the
    // Phase 1B inherent `to_single_vec` (parity oracle: the old
    // `Feature::to_single_vec` matrix path — both read the stored MultiPoint in
    // order and emit the same `[lat, lon]` list).
    let geometry = route::Query::get_one_koji(&conn.koji, id)
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;
    let route_points = koji_core::KojiGeometryCollection::new(vec![geometry]).to_single_vec();
    let mode = area_mode_for(&model.mode);

    let payload = RouteUpdated {
        dragonite_area_id: dragonite_area_id as i64,
        mode,
        route: route_points,
    };
    let event_id = EventDispatcher::publish(&conn.koji, TOPIC_ROUTE_UPDATED, &payload)
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;

    Ok(ApiResponse::success_with_status(
        StatusCode::ACCEPTED,
        json!({
            "route": model.id,
            "event_id": event_id.to_string(),
            "topic": TOPIC_ROUTE_UPDATED,
            "dragonite_area_id": dragonite_area_id,
            "mode": mode,
        }),
    ))
}

/// The `web::Scope` wiring the route handlers under `/routes`, mounted into
/// `/api/v2` by [`crate::start`].
pub fn scope() -> actix_web::Scope {
    web::scope("/routes")
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
