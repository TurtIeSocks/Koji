//! v2 typed CRUD for routes.
//!
//! Routes are geometry-bearing, so the `list`/`get_one` reads honor the
//! `?format=` return-type query and render through
//! [`respond_geo`](crate::utils::format::respond_geo): the GeoJSON shapes ride
//! inside the v2 envelope, the export formats (`sql`/`poracle`/…) come back raw.
//! Writes take lightly-typed snake_case DTOs ([`CreateRoute`] / [`PatchRoute`],
//! geojson `geometry` as a `serde_json::Value`), go through the koji-db `Query`,
//! and surface `201`+`Location` / `204` / `404` via
//! [`ServiceError`](crate::utils::error::ServiceError).
//!
//! Hand-written (not macro'd via [`super::resources`]) because the reads return
//! Koji-native geometry (`as_koji_collection` / `get_one_koji`) and carry the
//! `/publish` action — beyond the plain-JSON
//! [`koji_resource!`](macros::koji_resource) shape. (Routes have no hierarchy, so
//! they otherwise mirror [`super::geofences`].)

use actix_web::{HttpResponse, http::StatusCode, web};
use koji_db::{
    KojiDb,
    db::{geofence, route, sea_orm_active_enums::Mode},
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use utoipa::ToSchema;

use crate::requests::{ReturnTypeArg, get_return_type};
use crate::utils::error::ServiceError;
use crate::utils::{
    api_response::{ApiError, ApiResponse},
    format::respond_geo,
};
use koji_dragonite::AreaMode;
use koji_events::EventDispatcher;

use crate::dragonite::{RouteUpdated, TOPIC_ROUTE_UPDATED};

/// Query args for the geometry reads: the `?format=` return-type selector (with
/// `?rt=` kept as a one-release back-compat alias). Deliberately omits the legacy
/// `?internal=` flag (spec A8).
#[derive(Debug, Default, Deserialize)]
struct ReadQuery {
    /// Return-type selector; `rt` is the legacy spelling, folded in as a fallback.
    format: Option<String>,
    rt: Option<String>,
}

impl ReadQuery {
    /// The negotiated return type, defaulting to `default` when neither
    /// `?format=` nor `?rt=` is supplied.
    fn return_type(&self, default: ReturnTypeArg) -> ReturnTypeArg {
        match self.format.clone().or_else(|| self.rt.clone()) {
            Some(s) => get_return_type(s, &default),
            None => default,
        }
    }
}

/// Lightly-typed create body. Scalars + links are typed (boundary `400`s);
/// `geometry` rides as raw geojson. snake_case wire — koji-db's `to_route` keys
/// on snake (a camelCase rename would silently drop fields on write).
#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub(crate) struct CreateRoute {
    pub geofence_id: u32,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// GeoJSON geometry (`MultiPoint` route path).
    #[schema(value_type = Object)]
    pub geometry: serde_json::Value,
}

/// Lightly-typed patch body: every field optional, omitted fields dropped from
/// the serialized upsert value.
#[derive(Debug, Deserialize, Serialize, Default, ToSchema)]
#[serde(default)]
pub(crate) struct PatchRoute {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub geofence_id: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Option<Object>)]
    pub geometry: Option<serde_json::Value>,
}

/// `GET /api/v2/routes` — list all routes as a `FeatureCollection`, honoring
/// `?format=` (defaults to `featurecollection`).
#[utoipa::path(
    get,
    path = "/api/v2/routes",
    tag = "routes",
    params(("format" = Option<String>, Query, description = "Return type (default `featurecollection`)")),
    responses(
        (status = 200, description = "Routes (GeoJSON in the envelope, or a raw export format)", body = Object),
    ),
)]
async fn list(
    conn: web::Data<KojiDb>,
    query: web::Query<ReadQuery>,
) -> Result<HttpResponse, ServiceError> {
    let return_type = query.return_type(ReturnTypeArg::FeatureCollection);

    let coll = route::Query::as_koji_collection(&conn.koji).await?;

    Ok(respond_geo(coll, return_type))
}

/// `POST /api/v2/routes` — create a route → `201` + `Location`.
#[utoipa::path(
    post,
    path = "/api/v2/routes",
    tag = "routes",
    request_body = CreateRoute,
    responses(
        (status = 201, description = "Created; `Location` header points at the new route", body = Object),
        (status = 500, description = "Internal error", body = ApiError),
    ),
)]
pub(crate) async fn create(
    conn: web::Data<KojiDb>,
    hub: web::Data<crate::internal::realtime::RealtimeHub>,
    body: web::Json<CreateRoute>,
) -> Result<HttpResponse, ServiceError> {
    let value = serde_json::to_value(body.into_inner()).map_err(ServiceError::internal)?;
    let record = route::Query::upsert_json_return(&conn.koji, 0, value).await?;
    let id = record
        .get("id")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0);
    for (t, ev) in crate::internal::realtime::topics::created("route", id as i64) {
        hub.publish(&t, ev);
    }
    emit_route_updated(&conn.koji, id, &record).await;
    Ok(HttpResponse::build(StatusCode::CREATED)
        .insert_header(("Location", format!("/api/v2/routes/{id}")))
        .json(ApiResponse::Ok {
            data: record,
            meta: None,
        }))
}

/// `GET /api/v2/routes/{id}` — one route (by id or name) as a feature, honoring
/// `?format=` (defaults to `feature`); a missing route → `404`.
#[utoipa::path(
    get,
    path = "/api/v2/routes/{id}",
    tag = "routes",
    params(
        ("id" = String, Path, description = "Route id or name"),
        ("format" = Option<String>, Query, description = "Return type (default `feature`)"),
    ),
    responses(
        (status = 200, description = "The route (GeoJSON in the envelope, or a raw export format)", body = Object),
        (status = 404, description = "No such route", body = ApiError),
    ),
)]
async fn get_one(
    conn: web::Data<KojiDb>,
    path: web::Path<String>,
    query: web::Query<ReadQuery>,
) -> Result<HttpResponse, ServiceError> {
    let id = path.into_inner();
    let return_type = query.return_type(ReturnTypeArg::Feature);

    let geometry = route::Query::get_one_koji(&conn.koji, id.clone())
        .await
        .map_err(|_| ServiceError::NotFound {
            field: "route",
            message: format!("no route {id}"),
        })?;

    Ok(respond_geo(
        koji_core::KojiGeometryCollection::new(vec![geometry]),
        return_type,
    ))
}

/// `PATCH /api/v2/routes/{id}` — update a route by id → `200` envelope; `404` on
/// a missing id.
#[utoipa::path(
    patch,
    path = "/api/v2/routes/{id}",
    tag = "routes",
    params(("id" = u32, Path, description = "Route id")),
    request_body = PatchRoute,
    responses(
        (status = 200, description = "Updated route record", body = Object),
        (status = 404, description = "No such route", body = ApiError),
    ),
)]
async fn update(
    conn: web::Data<KojiDb>,
    hub: web::Data<crate::internal::realtime::RealtimeHub>,
    path: web::Path<u32>,
    body: web::Json<PatchRoute>,
) -> Result<HttpResponse, ServiceError> {
    let id = path.into_inner();
    // Fetch existing row — 404 on missing (also supplies the base for the merge).
    let existing = route::Query::get_one(&conn.koji, id.to_string())
        .await
        .map_err(|_| ServiceError::NotFound {
            field: "route",
            message: format!("no route {id}"),
        })?;
    // Build a full JSON from the existing model, then overlay only the fields
    // the PATCH body supplied (skip_serializing_if = "Option::is_none" ensures
    // absent fields are absent from the patch value).
    let mut merged = serde_json::to_value(&existing).map_err(ServiceError::internal)?;
    let patch_value = serde_json::to_value(body.into_inner()).map_err(ServiceError::internal)?;
    if let (Some(base), Some(patch)) = (merged.as_object_mut(), patch_value.as_object()) {
        for (k, v) in patch {
            base.insert(k.clone(), v.clone());
        }
    }
    let record = route::Query::upsert_json_return(&conn.koji, id, merged).await?;
    for (t, ev) in crate::internal::realtime::topics::updated("route", id as i64, record.clone()) {
        hub.publish(&t, ev);
    }
    emit_route_updated(&conn.koji, id as u64, &record).await;
    Ok(ApiResponse::success(record))
}

/// Emit `route.updated` to the outbox. `record` is the flat `route::Model`
/// JSON returned by `upsert_json_return` (top-level `name`/`geofence_id`).
/// `projectIds[]` is resolved through the route's PARENT geofence (routes
/// have no direct project link of their own) via the same
/// `geofence_project` helper `geofences.rs` uses.
async fn emit_route_updated(db: &sea_orm::DatabaseConnection, id: u64, record: &serde_json::Value) {
    let geofence_id = record
        .get("geofence_id")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or_default() as u32;
    let project_ids =
        koji_db::db::geofence_project::Query::project_ids_for_geofence(db, geofence_id)
            .await
            .unwrap_or_default();
    crate::utils::outbox::emit_event(
        db,
        "route.updated",
        serde_json::json!({
            "routeId": id,
            "geofenceId": geofence_id,
            "name": record.get("name").cloned().unwrap_or(serde_json::Value::Null),
            "projectIds": project_ids,
        }),
    )
    .await;
}

/// `DELETE /api/v2/routes/{id}` — `204 No Content`; `404` on a missing id.
#[utoipa::path(
    delete,
    path = "/api/v2/routes/{id}",
    tag = "routes",
    params(("id" = u32, Path, description = "Route id")),
    responses(
        (status = 204, description = "Deleted"),
        (status = 404, description = "No such route", body = ApiError),
    ),
)]
async fn remove(
    conn: web::Data<KojiDb>,
    hub: web::Data<crate::internal::realtime::RealtimeHub>,
    path: web::Path<u32>,
) -> Result<HttpResponse, ServiceError> {
    let id = path.into_inner();
    let result = route::Query::delete(&conn.koji, id).await?;
    if result.rows_affected == 0 {
        return Err(ServiceError::NotFound {
            field: "route",
            message: "does not exist".to_string(),
        });
    }
    for (t, ev) in crate::internal::realtime::topics::deleted("route", id as i64) {
        hub.publish(&t, ev);
    }
    Ok(HttpResponse::build(StatusCode::NO_CONTENT).finish())
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
/// unlinked geofence → `422`); an unknown route → `404`.
#[utoipa::path(
    post,
    path = "/api/v2/routes/{id}/publish",
    tag = "routes",
    params(("id" = String, Path, description = "Route id or name")),
    responses(
        (status = 202, description = "Publish event enqueued: `{ event_id }`", body = Object),
        (status = 404, description = "No such route", body = ApiError),
        (status = 422, description = "Route's geofence is not linked to a Dragonite area", body = ApiError),
    ),
)]
async fn publish(
    conn: web::Data<KojiDb>,
    path: web::Path<String>,
) -> Result<HttpResponse, ServiceError> {
    let id = path.into_inner();

    let model = route::Query::get_one(&conn.koji, id.clone())
        .await
        .map_err(|_| ServiceError::NotFound {
            field: "route",
            message: format!("no route {id}"),
        })?;

    // Linkage flows through the route's geofence.
    let fence = geofence::Query::get_one(&conn.koji, model.geofence_id.to_string()).await?;
    let Some(dragonite_area_id) = fence.dragonite_area_id else {
        return Err(ServiceError::Unprocessable {
            field: Some("dragonite_area_id".to_string()),
            message: "route's geofence is not linked to a Dragonite area".to_string(),
        });
    };

    // Route points as a Koji `SingleVec` (`[lat, lon]`). Fetch the route as a
    // `KojiGeometry`, wrap it in a one-item collection, and flatten via the
    // Phase 1B inherent `to_single_vec` (parity oracle: the old
    // `Feature::to_single_vec` matrix path — both read the stored MultiPoint in
    // order and emit the same `[lat, lon]` list).
    let geometry = route::Query::get_one_koji(&conn.koji, id).await?;
    let route_points = koji_core::KojiGeometryCollection::new(vec![geometry]).to_single_vec();
    let mode = area_mode_for(&model.mode);

    let payload = RouteUpdated {
        dragonite_area_id: dragonite_area_id as i64,
        mode,
        route: route_points,
    };
    let event_id = EventDispatcher::publish(&conn.koji, TOPIC_ROUTE_UPDATED, &payload)
        .await
        .map_err(ServiceError::internal)?;

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
pub(crate) fn scope() -> actix_web::Scope {
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

/// Like [`scope`] but omits the collection `GET` (list). Used by the `/internal`
/// scope, which overrides `GET /internal/routes` with the bespoke row-list
/// handler while forwarding all other route operations unchanged.
pub(crate) fn internal_item_scope() -> actix_web::Scope {
    web::scope("/routes")
        .service(web::resource("/{id}/publish").route(web::post().to(publish)))
        .service(
            web::resource("/{id}")
                .route(web::get().to(get_one))
                .route(web::patch().to(update))
                .route(web::delete().to(remove)),
        )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn sample_geometry() -> serde_json::Value {
        json!({
            "type": "MultiPoint",
            "coordinates": [[1.0, 2.0], [3.0, 4.0]]
        })
    }

    #[test]
    fn create_route_deserializes_snake_case_body() {
        let dto: CreateRoute = serde_json::from_value(json!({
            "geofence_id": 7,
            "name": "Patrol",
            "mode": "pokemon",
            "description": "loop",
            "geometry": sample_geometry()
        }))
        .unwrap();
        assert_eq!(dto.geofence_id, 7);
        assert_eq!(dto.name, "Patrol");
        assert_eq!(dto.mode.as_deref(), Some("pokemon"));
        assert_eq!(dto.description.as_deref(), Some("loop"));
        assert_eq!(dto.geometry["type"], "MultiPoint");
    }

    #[test]
    fn create_route_requires_geofence_id_name_geometry() {
        // `geofence_id` + `name` + `geometry` are required (not Option).
        assert!(
            serde_json::from_value::<CreateRoute>(
                json!({ "name": "x", "geometry": sample_geometry() })
            )
            .is_err()
        );
        assert!(
            serde_json::from_value::<CreateRoute>(
                json!({ "geofence_id": 1, "geometry": sample_geometry() })
            )
            .is_err()
        );
        assert!(
            serde_json::from_value::<CreateRoute>(json!({ "geofence_id": 1, "name": "x" }))
                .is_err()
        );
    }

    #[test]
    fn create_route_defaults_optionals() {
        let dto: CreateRoute = serde_json::from_value(json!({
            "geofence_id": 1,
            "name": "x",
            "geometry": sample_geometry()
        }))
        .unwrap();
        assert!(dto.mode.is_none());
        assert!(dto.description.is_none());
    }

    #[test]
    fn create_route_serializes_snake_keys_for_upsert() {
        // The serialized value feeds koji-db `to_route`, which reads snake keys.
        let dto = CreateRoute {
            geofence_id: 7,
            name: "n".into(),
            mode: None,
            description: None,
            geometry: sample_geometry(),
        };
        let v = serde_json::to_value(&dto).unwrap();
        assert_eq!(v["geofence_id"], 7);
        assert_eq!(v["name"], "n");
        assert!(v.get("geometry").is_some());
        // None optionals are omitted from the upsert value.
        assert!(v.get("mode").is_none());
        assert!(v.get("description").is_none());
    }

    #[test]
    fn patch_route_accepts_empty_body_all_none() {
        let dto: PatchRoute = serde_json::from_value(json!({})).unwrap();
        assert!(dto.geofence_id.is_none());
        assert!(dto.name.is_none());
        assert!(dto.mode.is_none());
        assert!(dto.description.is_none());
        assert!(dto.geometry.is_none());
    }

    #[test]
    fn patch_route_partial_omits_none_on_serialize() {
        let dto: PatchRoute =
            serde_json::from_value(json!({ "description": "only this" })).unwrap();
        assert_eq!(dto.description.as_deref(), Some("only this"));
        let v = serde_json::to_value(&dto).unwrap();
        assert_eq!(v["description"], "only this");
        assert!(v.get("geofence_id").is_none());
        assert!(v.get("name").is_none());
        assert!(v.get("mode").is_none());
        assert!(v.get("geometry").is_none());
    }

    #[test]
    fn read_query_format_takes_precedence_then_rt_then_default() {
        let q = ReadQuery {
            format: Some("sql".into()),
            rt: Some("feature".into()),
        };
        assert_eq!(
            q.return_type(ReturnTypeArg::FeatureCollection),
            ReturnTypeArg::Sql
        );

        let q = ReadQuery {
            format: None,
            rt: Some("sql".into()),
        };
        assert_eq!(
            q.return_type(ReturnTypeArg::FeatureCollection),
            ReturnTypeArg::Sql
        );

        let q = ReadQuery::default();
        assert_eq!(
            q.return_type(ReturnTypeArg::Feature),
            ReturnTypeArg::Feature
        );
    }
}
