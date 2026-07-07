//! v2 typed CRUD for geofences.
//!
//! Geofences are geometry-bearing, so the `list`/`get_one` reads honor the
//! `?format=` return-type query and render through
//! [`respond_geo`](crate::utils::format::respond_geo): the GeoJSON shapes ride
//! inside the v2 envelope, the export formats (`sql`/`poracle`/…) come back raw.
//! Writes take lightly-typed snake_case DTOs ([`CreateGeofence`] /
//! [`PatchGeofence`], geojson `geometry` as a `serde_json::Value`), go through
//! the koji-db `Query`, and surface `201`+`Location` / `204` / `404` via
//! [`ServiceError`](crate::utils::error::ServiceError).
//!
//! Hand-written (not macro'd via [`super::resources`]) because the reads return
//! Koji-native geometry (`get_all_koji` / `descendants` / `get_one_koji`) and
//! carry the `?depth`/`?level` geofence hierarchy + the `/publish` action —
//! beyond the plain-JSON [`koji_resource!`](macros::koji_resource) shape.

use actix_web::{HttpResponse, http::StatusCode, web};
use geojson::{Feature, Geometry};
use koji_db::{
    KojiDb,
    db::geofence::{self, Anchor, HierarchySpec},
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

use crate::dragonite::{GeofenceUpdated, TOPIC_GEOFENCE_UPDATED};

/// Query args for the geometry reads: the `?format=` return-type selector (with
/// `?rt=` kept as a one-release back-compat alias) plus the `?depth`/`?level`
/// hierarchy controls. Deliberately omits the legacy `?internal=` flag (spec
/// A8). `depth`+`level` together are a client error (400) — see [`list`].
#[derive(Debug, Default, Deserialize)]
struct ReadQuery {
    /// Return-type selector; `rt` is the legacy spelling, folded in as a fallback.
    format: Option<String>,
    rt: Option<String>,
    /// Cumulative subtree through `depth` levels (mutually exclusive with `level`).
    depth: Option<u32>,
    /// Exactly the geofences `level` levels below the anchor.
    level: Option<u32>,
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

    /// Resolve `?depth`/`?level` into a recursion spec, mapping the
    /// mutually-exclusive error to a `400`.
    // `ServiceError` is intentionally large (carries `DbErr`/`ModelError` by
    // value) — matches the crate-wide allow in `utils::error`.
    #[allow(clippy::result_large_err)]
    fn hierarchy(&self) -> Result<Option<HierarchySpec>, ServiceError> {
        HierarchySpec::from_args(self.depth, self.level).map_err(|_| ServiceError::Invalid {
            field: Some("hierarchy".to_string()),
            message: "depth and level are mutually exclusive".to_string(),
        })
    }
}

/// Lightly-typed create body. Scalars + links are typed (boundary `400`s);
/// `geometry` rides as raw geojson, and `projects`/`properties` as opaque arrays
/// the koji-db upsert reads through. snake_case wire — koji-db's `to_geofence`
/// and the `upsert_related_*` readers all key on snake (a camelCase rename would
/// silently drop fields on write).
#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub(crate) struct CreateGeofence {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
    /// GeoJSON geometry (`Polygon`/`MultiPolygon`/…).
    #[schema(value_type = Object)]
    pub geometry: serde_json::Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<u32>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[schema(value_type = Vec<Object>)]
    pub projects: Vec<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[schema(value_type = Vec<Object>)]
    pub properties: Vec<serde_json::Value>,
}

/// Lightly-typed patch body: every field optional, omitted fields dropped from
/// the serialized upsert value.
#[derive(Debug, Deserialize, Serialize, Default, ToSchema)]
#[serde(default)]
pub(crate) struct PatchGeofence {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Option<Object>)]
    pub geometry: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Option<Vec<Object>>)]
    pub projects: Option<Vec<serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Option<Vec<Object>>)]
    pub properties: Option<Vec<serde_json::Value>>,
}

/// `GET /api/v2/geofences` — list all geofences as a `FeatureCollection`,
/// honoring `?format=` (defaults to `featurecollection`).
///
/// With `?depth=N` or `?level=N` this becomes a recursive forest walk
/// (anchor = every `parent IS NULL` root): `depth=N` is the cumulative subtree
/// through level N, `level=N` is only the geofences exactly N levels down. The
/// two are mutually exclusive (both → 400). Omitted → the existing
/// non-recursive listing.
#[utoipa::path(
    get,
    path = "/api/v2/geofences",
    tag = "geofences",
    params(
        ("format" = Option<String>, Query, description = "Return type (default `featurecollection`)"),
        ("depth" = Option<u32>, Query, description = "Cumulative subtree through N levels (mutually exclusive with `level`)"),
        ("level" = Option<u32>, Query, description = "Exactly N levels below the anchor"),
    ),
    responses(
        (status = 200, description = "Geofences (GeoJSON in the envelope, or a raw export format)", body = Object),
        (status = 400, description = "`depth` and `level` are mutually exclusive", body = ApiError),
    ),
)]
async fn list(
    conn: web::Data<KojiDb>,
    query: web::Query<ReadQuery>,
) -> Result<HttpResponse, ServiceError> {
    let return_type = query.return_type(ReturnTypeArg::FeatureCollection);

    let coll = match query.hierarchy()? {
        Some(spec) => geofence::Query::descendants(&conn.koji, Anchor::Forest, spec).await?,
        None => geofence::Query::get_all_koji(&conn.koji).await?,
    };

    Ok(respond_geo(coll, return_type))
}

/// `POST /api/v2/geofences` — create a geofence → `201` + `Location`.
#[utoipa::path(
    post,
    path = "/api/v2/geofences",
    tag = "geofences",
    request_body = CreateGeofence,
    responses(
        (status = 201, description = "Created; `Location` header points at the new geofence", body = Object),
        (status = 500, description = "Internal error", body = ApiError),
    ),
)]
pub(crate) async fn create(
    conn: web::Data<KojiDb>,
    hub: web::Data<crate::internal::realtime::RealtimeHub>,
    body: web::Json<CreateGeofence>,
) -> Result<HttpResponse, ServiceError> {
    let value = serde_json::to_value(body.into_inner()).map_err(ServiceError::internal)?;
    let record = geofence::Query::upsert_json_return(&conn.koji, 0, value).await?;
    let id = record
        .get("id")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0);
    for (t, ev) in crate::internal::realtime::topics::created("geofence", id as i64) {
        hub.publish(&t, ev);
    }
    emit_geofence_updated(&conn.koji, id, &record).await;
    Ok(HttpResponse::build(StatusCode::CREATED)
        .insert_header(("Location", format!("/api/v2/geofences/{id}")))
        .json(ApiResponse::Ok {
            data: record,
            meta: None,
        }))
}

/// `GET /api/v2/geofences/{id}` — one geofence (by id or name) as a feature,
/// honoring `?format=` (defaults to `feature`).
///
/// With `?depth=N` or `?level=N` this anchors a recursive subtree on the named
/// geofence: `depth=N` is the cumulative subtree through level N, `level=N` is
/// only the geofences exactly N levels down. The two are mutually exclusive
/// (both → 400). Omitted → the existing single-geofence response; a missing
/// geofence → `404`.
#[utoipa::path(
    get,
    path = "/api/v2/geofences/{id}",
    tag = "geofences",
    params(
        ("id" = String, Path, description = "Geofence id or name"),
        ("format" = Option<String>, Query, description = "Return type (default `feature`)"),
        ("depth" = Option<u32>, Query, description = "Cumulative subtree through N levels"),
        ("level" = Option<u32>, Query, description = "Exactly N levels below the anchor"),
    ),
    responses(
        (status = 200, description = "The geofence (GeoJSON in the envelope, or a raw export format)", body = Object),
        (status = 400, description = "`depth` and `level` are mutually exclusive", body = ApiError),
        (status = 404, description = "No such geofence", body = ApiError),
    ),
)]
async fn get_one(
    conn: web::Data<KojiDb>,
    path: web::Path<String>,
    query: web::Query<ReadQuery>,
) -> Result<HttpResponse, ServiceError> {
    let id = path.into_inner();
    let return_type = query.return_type(ReturnTypeArg::Feature);

    let coll = match query.hierarchy()? {
        Some(spec) => {
            let anchor = id
                .parse::<u32>()
                .map(Anchor::Id)
                .unwrap_or_else(|_| Anchor::Name(id));
            geofence::Query::descendants(&conn.koji, anchor, spec).await?
        }
        None => {
            let geometry = geofence::Query::get_one_koji(&conn.koji, id.clone())
                .await
                .map_err(|_| ServiceError::NotFound {
                    field: "geofence",
                    message: format!("no geofence {id}"),
                })?;
            koji_core::KojiGeometryCollection::new(vec![geometry])
        }
    };

    Ok(respond_geo(coll, return_type))
}

/// `PATCH /api/v2/geofences/{id}` — update a geofence by id → `200` envelope;
/// `404` on a missing id.
#[utoipa::path(
    patch,
    path = "/api/v2/geofences/{id}",
    tag = "geofences",
    params(("id" = u32, Path, description = "Geofence id")),
    request_body = PatchGeofence,
    responses(
        (status = 200, description = "Updated geofence record", body = Object),
        (status = 404, description = "No such geofence", body = ApiError),
    ),
)]
async fn update(
    conn: web::Data<KojiDb>,
    hub: web::Data<crate::internal::realtime::RealtimeHub>,
    path: web::Path<u32>,
    body: web::Json<PatchGeofence>,
) -> Result<HttpResponse, ServiceError> {
    let id = path.into_inner();
    // Fetch existing row — 404 on missing (also supplies the base for the merge).
    let existing = geofence::Query::get_one(&conn.koji, id.to_string())
        .await
        .map_err(|_| ServiceError::NotFound {
            field: "geofence",
            message: format!("no geofence {id}"),
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
    let record = geofence::Query::upsert_json_return(&conn.koji, id, merged).await?;
    for (t, ev) in crate::internal::realtime::topics::updated("geofence", id as i64, record.clone()) {
        hub.publish(&t, ev);
    }
    emit_geofence_updated(&conn.koji, id as u64, &record).await;
    Ok(ApiResponse::success(record))
}

/// Emit `geofence.updated` to the outbox with the current `projectIds[]`
/// (resolved fresh from `geofence_project`, not from the upsert body — a
/// PATCH may not touch `projects` at all, so the join table is the source of
/// truth at emit time). `record` is the flat `geofence::Model` JSON returned
/// by `upsert_json_return` (top-level `name`, not nested under `properties` —
/// the geofence handlers return the raw row, not a GeoJSON Feature).
async fn emit_geofence_updated(db: &sea_orm::DatabaseConnection, id: u64, record: &serde_json::Value) {
    let project_ids = koji_db::db::geofence_project::Query::project_ids_for_geofence(db, id as u32)
        .await
        .unwrap_or_default();
    crate::utils::outbox::emit_event(
        db,
        "geofence.updated",
        serde_json::json!({
            "geofenceId": id,
            "name": record.get("name").cloned().unwrap_or(serde_json::Value::Null),
            "projectIds": project_ids,
        }),
    )
    .await;
}

/// `DELETE /api/v2/geofences/{id}` — `204 No Content`; `404` on a missing id.
#[utoipa::path(
    delete,
    path = "/api/v2/geofences/{id}",
    tag = "geofences",
    params(("id" = u32, Path, description = "Geofence id")),
    responses(
        (status = 204, description = "Deleted"),
        (status = 404, description = "No such geofence", body = ApiError),
    ),
)]
async fn remove(
    conn: web::Data<KojiDb>,
    hub: web::Data<crate::internal::realtime::RealtimeHub>,
    path: web::Path<u32>,
) -> Result<HttpResponse, ServiceError> {
    let id = path.into_inner();
    let result = geofence::Query::delete(&conn.koji, id).await?;
    if result.rows_affected == 0 {
        return Err(ServiceError::NotFound {
            field: "geofence",
            message: "does not exist".to_string(),
        });
    }
    for (t, ev) in crate::internal::realtime::topics::deleted("geofence", id as i64) {
        hub.publish(&t, ev);
    }
    Ok(HttpResponse::build(StatusCode::NO_CONTENT).finish())
}

/// `POST /api/v2/geofences/{id}/publish` — publish a geofence's fence to its
/// linked Dragonite area.
///
/// Replaces the v1 GET-that-mutates `/geofence/push/{id}` and the old direct
/// controller-DB write (P5, architecture §7): instead of writing the golbat DB
/// inline, it appends an [`area.geofence_updated`](TOPIC_GEOFENCE_UPDATED) event
/// to the outbox, which the dispatcher delivers to the `DragoniteSubscriber`
/// (PATCH `/v2/areas/{id}`). Returns `202 { event_id }`.
///
/// Gated on linkage: a geofence with no `dragonite_area_id` yields `422` (it is
/// not bound to a Dragonite area, so there is nothing to push to); an unknown
/// geofence yields `404`.
#[utoipa::path(
    post,
    path = "/api/v2/geofences/{id}/publish",
    tag = "geofences",
    params(("id" = String, Path, description = "Geofence id or name")),
    responses(
        (status = 202, description = "Publish event enqueued: `{ event_id }`", body = Object),
        (status = 404, description = "No such geofence", body = ApiError),
        (status = 422, description = "Geofence is not linked to a Dragonite area", body = ApiError),
    ),
)]
async fn publish(
    conn: web::Data<KojiDb>,
    path: web::Path<String>,
) -> Result<HttpResponse, ServiceError> {
    let id = path.into_inner();

    // Resolve the geofence (by id or name) — 404 if it doesn't exist.
    let model = geofence::Query::get_one(&conn.koji, id.clone())
        .await
        .map_err(|_| ServiceError::NotFound {
            field: "geofence",
            message: format!("no geofence {id}"),
        })?;

    // Linkage gate: only linked geofences can be pushed.
    let Some(dragonite_area_id) = model.dragonite_area_id else {
        return Err(ServiceError::Unprocessable {
            field: Some("dragonite_area_id".to_string()),
            message: "geofence is not linked to a Dragonite area".to_string(),
        });
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
    let geometry =
        serde_json::from_value::<Geometry>(model.geometry.clone()).map_err(ServiceError::internal)?;
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
        .map_err(ServiceError::internal)?;

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

/// Reshape the related-read JSON (`{geometry, name, mode, parent, projects,
/// properties, ...}`) into a GeoJSON Feature: geometry lifted to the Feature
/// level, everything else kept in the `properties` bag so the admin client's
/// `featureToRecord` hydrates the related data without a data-provider change.
fn related_to_feature(mut related: serde_json::Value) -> serde_json::Value {
    let geometry = related
        .as_object_mut()
        .and_then(|o| o.remove("geometry"))
        .unwrap_or(serde_json::Value::Null);
    json!({ "type": "Feature", "geometry": geometry, "properties": related })
}

/// `GET /internal/geofences/{id}` — the full editable record as a GeoJSON
/// Feature whose `properties` bag carries the related `projects`/`properties`
/// (+ name/mode/parent) that the admin edit form hydrates. The public `get_one`
/// returns only id/mode/name in the feature, which is insufficient for editing.
pub(crate) async fn internal_get_one(
    conn: web::Data<KojiDb>,
    path: web::Path<u32>,
) -> Result<HttpResponse, ServiceError> {
    let id = path.into_inner();
    let related = geofence::Query::get_one_json_with_related(&conn.koji, id.to_string())
        .await
        .map_err(|_| ServiceError::NotFound {
            field: "geofence",
            message: format!("no geofence {id}"),
        })?;
    Ok(ApiResponse::success(related_to_feature(related)))
}

/// Like [`scope`] but omits the collection `GET` (list). Used by the `/internal`
/// scope, which overrides `GET /internal/geofences` with the bespoke row-list
/// handler while forwarding all other geofence operations unchanged.
///
/// Collection `POST` (create) + all `/{id}` routes (getOne / patch / delete /
/// publish / golbat-data) are forwarded as-is. Mounted at `/geofences` so the
/// caller's parent scope (`/internal`) contributes the full path prefix.
pub(crate) fn internal_item_scope() -> actix_web::Scope {
    web::scope("/geofences")
        .service(web::resource("/{id}/publish").route(web::post().to(publish)))
        .service(
            web::resource("/{id}/golbat-data")
                .route(web::get().to(crate::public::v2::golbat_data::golbat_data)),
        )
        .service(
            web::resource("/{id}")
                .route(web::get().to(internal_get_one))
                .route(web::patch().to(update))
                .route(web::delete().to(remove)),
        )
}

/// The `web::Scope` wiring the geofence handlers under `/geofences`, mounted into
/// `/api/v2` by [`crate::start`]. The `/{id}/publish` and `/{id}/golbat-data`
/// sub-resources are registered before the `/{id}` catch-all so the more specific
/// routes match first.
pub(crate) fn scope() -> actix_web::Scope {
    web::scope("/geofences")
        .service(
            web::resource("")
                .route(web::get().to(list))
                .route(web::post().to(create)),
        )
        .service(web::resource("/{id}/publish").route(web::post().to(publish)))
        // golbat-data as a geofence sub-resource (architecture §4.5): the
        // category's golbat points within the path `{id}` geofence. The handler
        // lives in `super::golbat_data` (re-keyed off the path id in P3).
        .service(
            web::resource("/{id}/golbat-data")
                .route(web::get().to(crate::public::v2::golbat_data::golbat_data)),
        )
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
            "type": "Polygon",
            "coordinates": [[[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 0.0]]]
        })
    }

    #[test]
    fn create_geofence_deserializes_snake_case_body() {
        let dto: CreateGeofence = serde_json::from_value(json!({
            "name": "Denver",
            "mode": "pokemon",
            "geometry": sample_geometry(),
            "parent": 7,
            "projects": [1, 2],
            "properties": [{ "name": "color", "value": "#fff" }]
        }))
        .unwrap();
        assert_eq!(dto.name, "Denver");
        assert_eq!(dto.mode.as_deref(), Some("pokemon"));
        assert_eq!(dto.parent, Some(7));
        assert_eq!(dto.geometry["type"], "Polygon");
        assert_eq!(dto.projects.len(), 2);
        assert_eq!(dto.properties.len(), 1);
    }

    #[test]
    fn create_geofence_requires_name_and_geometry() {
        // `name` + `geometry` are required (not Option) — a body missing them fails.
        assert!(
            serde_json::from_value::<CreateGeofence>(json!({ "geometry": sample_geometry() }))
                .is_err()
        );
        assert!(serde_json::from_value::<CreateGeofence>(json!({ "name": "x" })).is_err());
    }

    #[test]
    fn create_geofence_defaults_optional_collections() {
        let dto: CreateGeofence =
            serde_json::from_value(json!({ "name": "x", "geometry": sample_geometry() })).unwrap();
        assert!(dto.mode.is_none());
        assert!(dto.parent.is_none());
        assert!(dto.projects.is_empty());
        assert!(dto.properties.is_empty());
    }

    #[test]
    fn create_geofence_serializes_snake_keys_for_upsert() {
        // The serialized value feeds koji-db `to_geofence` + `upsert_related_*`,
        // which read snake keys. Empty optionals are dropped.
        let dto = CreateGeofence {
            name: "n".into(),
            mode: None,
            geometry: sample_geometry(),
            parent: None,
            projects: vec![],
            properties: vec![],
        };
        let v = serde_json::to_value(&dto).unwrap();
        assert_eq!(v["name"], "n");
        assert!(v.get("geometry").is_some());
        // Empty/None optionals are omitted from the upsert value.
        assert!(v.get("mode").is_none());
        assert!(v.get("parent").is_none());
        assert!(v.get("projects").is_none());
        assert!(v.get("properties").is_none());
    }

    #[test]
    fn patch_geofence_accepts_empty_body_all_none() {
        let dto: PatchGeofence = serde_json::from_value(json!({})).unwrap();
        assert!(dto.name.is_none());
        assert!(dto.mode.is_none());
        assert!(dto.geometry.is_none());
        assert!(dto.parent.is_none());
        assert!(dto.projects.is_none());
        assert!(dto.properties.is_none());
    }

    #[test]
    fn patch_geofence_partial_omits_none_on_serialize() {
        let dto: PatchGeofence = serde_json::from_value(json!({ "name": "renamed" })).unwrap();
        assert_eq!(dto.name.as_deref(), Some("renamed"));
        let v = serde_json::to_value(&dto).unwrap();
        assert_eq!(v["name"], "renamed");
        // Every other field is dropped from the serialized upsert value.
        assert!(v.get("mode").is_none());
        assert!(v.get("geometry").is_none());
        assert!(v.get("parent").is_none());
        assert!(v.get("projects").is_none());
        assert!(v.get("properties").is_none());
    }

    #[test]
    fn read_query_format_takes_precedence_then_rt_then_default() {
        let q = ReadQuery {
            format: Some("sql".into()),
            rt: Some("feature".into()),
            ..Default::default()
        };
        assert_eq!(
            q.return_type(ReturnTypeArg::FeatureCollection),
            ReturnTypeArg::Sql
        );

        let q = ReadQuery {
            format: None,
            rt: Some("sql".into()),
            ..Default::default()
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

    #[test]
    fn related_to_feature_moves_geometry_and_keeps_related() {
        let related = json!({
            "id": 9, "name": "F", "mode": "pokemon", "parent": null,
            "geometry": { "type": "Polygon", "coordinates": [] },
            "projects": [1, 2],
            "properties": [{ "property_id": 5, "value": true, "category": "boolean", "name": "is_event" }],
        });
        let feature = related_to_feature(related);
        assert_eq!(feature["type"], "Feature");
        // geometry is lifted out to the Feature level
        assert_eq!(feature["geometry"]["type"], "Polygon");
        // the related data rides in the properties bag (so featureToRecord hydrates it)
        assert_eq!(feature["properties"]["projects"], json!([1, 2]));
        assert_eq!(feature["properties"]["properties"][0]["property_id"], 5);
        assert_eq!(feature["properties"]["name"], "F");
        // geometry key is not duplicated inside properties (it was removed)
        assert!(feature["properties"].get("geometry").is_none_or(|g| g.is_null()));
    }

    #[test]
    fn read_query_hierarchy_both_is_400() {
        let q = ReadQuery {
            depth: Some(1),
            level: Some(2),
            ..Default::default()
        };
        let err = q.hierarchy().unwrap_err();
        assert!(matches!(err, ServiceError::Invalid { .. }));
    }
}
