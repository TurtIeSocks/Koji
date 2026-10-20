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
use geo::BoundingRect;
use geojson::{Feature, FeatureCollection, Geometry};
use koji_db::{
    KojiDb,
    db::geofence::{self, Anchor, HierarchySpec},
    db::geofence_project,
    query_args::DescendantSpec,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use utoipa::ToSchema;

use crate::requests::{ReturnTypeArg, negotiate_return_type};
use crate::utils::error::ServiceError;
use crate::utils::{
    api_response::{ApiError, ApiResponse},
    format::respond_geo,
};
use koji_db::db::{
    dragonite_link::{self, LinkChange},
    sea_orm_active_enums::Mode,
    webhook,
};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, TransactionTrait};

use super::geofence_link::plan_links;
use super::publish_plan::plan_publish;
use crate::dragonite::{
    GeofenceUpdated, TOPIC_GEOFENCE_UPDATED, area_mode_for, client_for, linked_targets, publish_to,
};

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
    /// Comma-separated geofence ids — scope the list to only these (`list` only).
    ids: Option<String>,
    /// `minLng,minLat,maxLng,maxLat` — scope the list to fences whose bbox
    /// overlaps this box (`list` only).
    bbox: Option<String>,
    /// Only fences with this mode (`unset|pokemon|fort|quest`). Requires `bbox`.
    mode: Option<String>,
    /// Comma-separated project ids — only fences linked to ANY of these
    /// projects. Requires `bbox`.
    projects: Option<String>,
    /// Levels of descendants to add below the fences `projects` selects (at
    /// most 10). Requires `projects`.
    descendants: Option<u32>,
    /// Comma-separated modes limiting which descendants are added. Requires
    /// `descendants`.
    descendant_modes: Option<String>,
}

impl ReadQuery {
    /// The negotiated return type, defaulting to `default` when neither
    /// `?format=` nor `?rt=` is supplied.
    fn return_type(&self, default: ReturnTypeArg) -> ReturnTypeArg {
        negotiate_return_type(self.format.as_deref(), self.rt.as_deref(), default)
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

    /// Parse `?ids=1,2,3` into distinct u32 ids: split on `,`, trim, skip blank
    /// segments. A non-numeric segment is a client error (400); an id with no
    /// matching row is not (silently omitted downstream).
    #[allow(clippy::result_large_err)]
    fn ids(&self) -> Result<Option<Vec<u32>>, ServiceError> {
        let Some(raw) = &self.ids else {
            return Ok(None);
        };
        let ids = raw
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| {
                s.parse::<u32>().map_err(|_| ServiceError::Invalid {
                    field: Some("ids".to_string()),
                    message: format!("invalid geofence id: {s:?}"),
                })
            })
            .collect::<Result<Vec<u32>, ServiceError>>()?;
        Ok(Some(ids))
    }

    /// Parse `?bbox=minLng,minLat,maxLng,maxLat` (the geojson bbox-member
    /// order). Anything other than exactly 4 parseable, finite numbers is a
    /// client error (400) — `nan`/`inf` parse fine as `f64` but would make the
    /// AABB overlap test always false, silently disabling the filter instead
    /// of erroring.
    #[allow(clippy::result_large_err)]
    fn bbox(&self) -> Result<Option<[f64; 4]>, ServiceError> {
        let Some(raw) = &self.bbox else {
            return Ok(None);
        };
        let invalid = || ServiceError::Invalid {
            field: Some("bbox".to_string()),
            message: "bbox must be minLng,minLat,maxLng,maxLat".to_string(),
        };
        let parts: Vec<&str> = raw.split(',').map(str::trim).collect();
        if parts.len() != 4 {
            return Err(invalid());
        }
        let mut out = [0.0_f64; 4];
        for (slot, part) in out.iter_mut().zip(parts.iter()) {
            let v = part.parse::<f64>().map_err(|_| invalid())?;
            if !v.is_finite() {
                return Err(invalid());
            }
            *slot = v;
        }
        Ok(Some(out))
    }

    /// Parse `?mode=` into the storage enum. Unknown strings are a 400.
    #[allow(clippy::result_large_err)]
    fn mode_filter(&self) -> Result<Option<koji_db::db::sea_orm_active_enums::Mode>, ServiceError> {
        use koji_db::db::sea_orm_active_enums::Mode;
        let Some(raw) = &self.mode else {
            return Ok(None);
        };
        match raw.trim().to_ascii_lowercase().as_str() {
            "unset" => Ok(Some(Mode::Unset)),
            "pokemon" => Ok(Some(Mode::Pokemon)),
            "fort" => Ok(Some(Mode::Fort)),
            "quest" => Ok(Some(Mode::Quest)),
            other => Err(ServiceError::Invalid {
                field: Some("mode".to_string()),
                message: format!("invalid mode: {other:?}"),
            }),
        }
    }

    /// Resolve `?descendants`/`?descendant_modes`; a refused pair is a `400`.
    #[allow(clippy::result_large_err)]
    fn descendant_spec(&self) -> Result<Option<DescendantSpec>, ServiceError> {
        Ok(DescendantSpec::from_args(
            self.descendants,
            self.descendant_modes.as_deref(),
        )?)
    }

    /// Parse `?projects=1,2` exactly like `ids()` (trim, skip blanks, 400 on
    /// a non-numeric segment).
    #[allow(clippy::result_large_err)]
    fn project_ids(&self) -> Result<Option<Vec<u32>>, ServiceError> {
        let Some(raw) = &self.projects else {
            return Ok(None);
        };
        let ids = raw
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| {
                s.parse::<u32>().map_err(|_| ServiceError::Invalid {
                    field: Some("projects".to_string()),
                    message: format!("invalid project id: {s:?}"),
                })
            })
            .collect::<Result<Vec<u32>, ServiceError>>()?;
        Ok(Some(ids))
    }
}

/// A feature's own bbox (`[minLng, minLat, maxLng, maxLat]`), via the same
/// geojson->geo conversion `koji_core::KojiGeometry::bbox()` is built on —
/// `None` for a feature with no geometry or one that fails the conversion
/// (such a feature is dropped by [`features_intersecting_bbox`] rather than
/// surfacing an error).
#[allow(dead_code)] // kept as the reference/fallback impl below `features_intersecting_bbox`
fn feature_bbox(f: &Feature) -> Option<[f64; 4]> {
    let geom = f.geometry.as_ref()?;
    let g = geo::Geometry::<koji_core::Precision>::try_from(geom).ok()?;
    let rect = g.bounding_rect()?;
    Some([rect.min().x, rect.min().y, rect.max().x, rect.max().y])
}

/// Standard AABB overlap test (`a`/`b` both `[minX, minY, maxX, maxY]`);
/// touching counts as overlapping, so a bbox that only straddles the boundary
/// still intersects. This is the semantics `geofence::Query::get_koji_by_bbox`
/// (koji-db) mirrors with `<=`/`>=` column comparisons — kept here (with its
/// unit tests below) as the pinned, pure reference for that DB predicate.
#[allow(dead_code)] // reference/pinning only now — see `features_intersecting_bbox` below
fn bbox_overlaps(a: [f64; 4], b: [f64; 4]) -> bool {
    !(a[2] < b[0] || a[0] > b[2] || a[3] < b[1] || a[1] > b[3])
}

/// Keep only the features whose own bbox overlaps `bbox`
/// (`[minLng, minLat, maxLng, maxLat]`). Pure — no DB, no network — so it unit
/// tests without a database. `list`'s `?bbox=` path now calls the DB-level
/// `geofence::Query::get_koji_by_bbox` instead (indexed columns beat an
/// in-memory scan of every row); this helper is kept, unused in the request
/// path, purely as the pinned reference for that predicate's semantics and as
/// a manual fallback if the DB path is ever bypassed.
#[allow(dead_code)] // superseded by geofence::Query::get_koji_by_bbox; kept as reference/fallback
fn features_intersecting_bbox(fc: FeatureCollection, bbox: [f64; 4]) -> FeatureCollection {
    let features = fc
        .features
        .into_iter()
        .filter(|f| feature_bbox(f).is_some_and(|fb| bbox_overlaps(fb, bbox)))
        .collect();
    FeatureCollection {
        bbox: fc.bbox,
        features,
        foreign_members: fc.foreign_members,
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
    /// Dragonite Areas this geofence publishes to, one change per Dragonite
    /// webhook: a number links it to that Area on that webhook, `null` unlinks
    /// it. A webhook the list leaves out keeps its link.
    #[serde(skip_serializing)]
    #[schema(value_type = Option<Vec<DragoniteLinkChange>>)]
    pub dragonite_links: Option<Vec<DragoniteLinkChange>>,
}

/// One change to a geofence's links: its Area on one Dragonite webhook.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, ToSchema)]
pub(crate) struct DragoniteLinkChange {
    /// The Dragonite webhook (a `webhook_subscription` row in `dragonite` mode).
    pub webhook_id: u64,
    /// The Area id on that Dragonite; `null` removes the link. Required, so a
    /// misspelled key cannot unlink by omission.
    #[serde(deserialize_with = "required_nullable")]
    #[schema(value_type = Option<u32>)]
    pub dragonite_area_id: Option<u32>,
}

impl From<DragoniteLinkChange> for LinkChange {
    fn from(change: DragoniteLinkChange) -> Self {
        LinkChange {
            webhook_id: change.webhook_id,
            dragonite_area_id: change.dragonite_area_id,
        }
    }
}

/// A field that must be present but may be `null`.
fn required_nullable<'de, D, T>(de: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(de)
}

/// Body of `POST /api/v2/geofences/link-dragonite`.
#[derive(Debug, Deserialize, ToSchema)]
pub(crate) struct LinkDragonite {
    /// The Dragonite webhook to link on. Only the geofences it reaches are
    /// considered.
    pub webhook_id: u64,
    /// Answer what would be linked without writing anything.
    #[serde(default)]
    pub dry_run: bool,
}

/// Body of `POST /api/v2/geofences/publish`.
#[derive(Debug, Deserialize, ToSchema)]
pub(crate) struct PublishGeofences {
    /// The geofences to publish, by id.
    pub ids: Vec<u32>,
    /// Answer the writes and the skipped geofences without sending anything.
    #[serde(default)]
    pub dry_run: bool,
    /// The writes the caller was shown and approved. When given on a send, they
    /// must equal the writes planned now, else nothing is sent and the answer is
    /// `409` with the current plan.
    #[serde(default)]
    pub writes: Option<Vec<ApprovedWrite>>,
}

/// One write the caller approved: a geofence to one Dragonite's Area.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize, ToSchema)]
pub(crate) struct ApprovedWrite {
    pub geofence: u32,
    pub webhook_id: u64,
    pub dragonite_area_id: u32,
}

/// `?descendants` reads below the fences a project holds, so it needs the
/// `projects` filter (and `bbox`, which `projects` needs).
fn descendants_need_projects() -> ServiceError {
    ServiceError::Invalid {
        field: Some("descendants".to_string()),
        message: "descendants needs the projects and bbox filters".to_string(),
    }
}

/// `GET /api/v2/geofences` — list all geofences as a `FeatureCollection`,
/// honoring `?format=` (defaults to `featurecollection`).
///
/// `?ids=1,2,3` scopes the read to only those geofences (a single indexed
/// `WHERE id IN (...)` query — no full-table read). `?bbox=minLng,minLat,
/// maxLng,maxLat` scopes it to fences whose persisted bbox columns
/// (`min_lat`/`min_lng`/`max_lat`/`max_lng`) overlap the box — a DB-level
/// `WHERE` (see [`geofence::Query::get_koji_by_bbox`]), no full-table read
/// either. `bbox` is checked ahead of `ids` — supplying **both** is a `400`
/// (not "ids silently wins"), a deliberate reorder from the earlier
/// ids-then-bbox precedence: a silently-dropped filter reads as data loss.
///
/// `?mode=` and `?projects=1,2` are **bbox refinements**: only meaningful
/// alongside `?bbox=`, so either one supplied without `bbox` is a `400`.
/// `?mode=` (`unset|pokemon|fort|quest`) further restricts the bbox result to
/// that exact mode. `?projects=` further restricts it to fences linked to ANY
/// of the given project ids (resolved via [`geofence_project::Query::
/// geofence_ids_for_projects`]) — a project with no linked fences is a real,
/// empty filter (returns nothing), not treated the same as no filter at all.
///
/// `?descendants=N` (at most 10, with `?projects=` and `?bbox=`) adds the
/// children of the fences those filters select, then their children, N levels
/// down, read from the whole table. `?descendant_modes=pokemon,fort` limits
/// which modes are added (default every mode). A fence appears once, however
/// it got in. `?mode=` and the bbox only choose the starting set.
///
/// With `?depth=N` or `?level=N` (and no `ids`/`bbox`) this becomes a
/// recursive forest walk (anchor = every `parent IS NULL` root): `depth=N` is
/// the cumulative subtree through level N, `level=N` is only the geofences
/// exactly N levels down. The two are mutually exclusive (both → 400).
/// Omitted → the existing non-recursive listing.
#[utoipa::path(
    get,
    path = "/api/v2/geofences",
    tag = "geofences",
    params(
        ("format" = Option<String>, Query, description = "Return type (default `featurecollection`)"),
        ("ids" = Option<String>, Query, description = "Comma-separated geofence ids — only those fences. 400 if combined with `bbox`"),
        ("bbox" = Option<String>, Query, description = "minLng,minLat,maxLng,maxLat — only fences whose bbox overlaps"),
        ("mode" = Option<String>, Query, description = "unset|pokemon|fort|quest — bbox refinement, requires `bbox` (400 otherwise)"),
        ("projects" = Option<String>, Query, description = "Comma-separated project ids — bbox refinement, requires `bbox` (400 otherwise)"),
        ("descendants" = Option<u32>, Query, description = "Levels of descendants (max 10) to add below the fences `projects` selects, read from the whole table. Requires `projects`"),
        ("descendant_modes" = Option<String>, Query, description = "Comma-separated modes (`unset|pokemon|fort|quest`) limiting which descendants are added. Requires `descendants`"),
        ("depth" = Option<u32>, Query, description = "Cumulative subtree through N levels (mutually exclusive with `level`)"),
        ("level" = Option<u32>, Query, description = "Exactly N levels below the anchor"),
    ),
    responses(
        (status = 200, description = "Geofences (GeoJSON in the envelope, or a raw export format)", body = Object),
        (status = 400, description = "`depth`/`level` mutually exclusive; `ids`+`bbox` together; `mode`/`projects` malformed or supplied without `bbox`; `descendants` above 10, without `projects`, or `descendant_modes` unknown or without `descendants`", body = ApiError),
    ),
)]
async fn list(
    conn: web::Data<KojiDb>,
    query: web::Query<ReadQuery>,
) -> Result<HttpResponse, ServiceError> {
    let return_type = query.return_type(ReturnTypeArg::FeatureCollection);
    let mode = query.mode_filter()?;
    let project_ids = query.project_ids()?;
    let descendants = query.descendant_spec()?;

    if let Some(bbox) = query.bbox()? {
        if query.ids.is_some() {
            return Err(ServiceError::Invalid {
                field: Some("bbox".to_string()),
                message: "ids and bbox are mutually exclusive".to_string(),
            });
        }
        let id_scope = match project_ids {
            Some(pids) => {
                Some(geofence_project::Query::geofence_ids_for_projects(&conn.koji, &pids).await?)
            }
            None => None,
        };
        if descendants.is_some() && id_scope.is_none() {
            return Err(descendants_need_projects());
        }
        let mut coll = geofence::Query::get_koji_by_bbox(&conn.koji, bbox, mode, id_scope).await?;
        // Descendants come from the whole table, whatever chose the starting set.
        if let Some(spec) = descendants {
            let members: Vec<u32> = coll.items.iter().filter_map(|item| item.meta.id).collect();
            let added = geofence::Query::descendant_ids(&conn.koji, &members, &spec).await?;
            if !added.is_empty() {
                coll.items.extend(
                    geofence::Query::get_koji_by_ids(&conn.koji, &added)
                        .await?
                        .items,
                );
            }
        }
        return Ok(respond_geo(coll, return_type));
    }

    // mode/projects are bbox refinements — using them anywhere else is a 400,
    // not a silent no-op (a silently-ignored filter looks like data loss).
    // Attribute the error to a param the caller actually sent (both present →
    // "mode", arbitrarily but deterministically).
    if mode.is_some() || query.projects.is_some() {
        let field = if mode.is_some() { "mode" } else { "projects" };
        return Err(ServiceError::Invalid {
            field: Some(field.to_string()),
            message: "mode/projects filters require bbox".to_string(),
        });
    }

    if descendants.is_some() {
        return Err(descendants_need_projects());
    }

    if let Some(ids) = query.ids()? {
        let coll = geofence::Query::get_koji_by_ids(&conn.koji, &ids).await?;
        return Ok(respond_geo(coll, return_type));
    }

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
    let carries_projects = value.get("projects").is_some();
    let record = geofence::Query::upsert_json_return(&conn.koji, 0, value).await?;
    let id = super::crud::record_id(&record)?;
    for (t, ev) in crate::internal::realtime::topics::created("geofence", id as i64) {
        hub.publish(&t, ev);
    }
    emit_geofence_updated(&conn.koji, id, &record).await;
    // Membership diff only when the body carried `projects` (create starts
    // from an empty `before`, since the fence didn't exist a moment ago).
    if carries_projects {
        emit_geofence_membership_diff(&conn.koji, id as u32, &[]).await;
    }
    Ok(super::crud::created_response(
        "/api/v2/geofences",
        id,
        record,
    ))
}

/// `GET /api/v2/geofences/{id}` — one geofence (by id or name) as a feature,
/// honoring `?format=` (defaults to `feature`).
///
/// With `?depth=N` or `?level=N` this anchors a recursive subtree on the named
/// geofence: `depth=N` is the cumulative subtree through level N, `level=N` is
/// only the geofences exactly N levels down. The two are mutually exclusive
/// (both → 400). Omitted → the existing single-geofence response; a missing
/// geofence → `404`. A name held by several modes needs `?mode=`, else `409`
/// naming the modes.
#[utoipa::path(
    get,
    path = "/api/v2/geofences/{id}",
    tag = "geofences",
    params(
        ("id" = String, Path, description = "Geofence id or name"),
        ("mode" = Option<String>, Query, description = "Pick the geofence of this mode when `id` is a name shared by several modes (`unset|pokemon|fort|quest`)"),
        ("format" = Option<String>, Query, description = "Return type (default `feature`)"),
        ("depth" = Option<u32>, Query, description = "Cumulative subtree through N levels"),
        ("level" = Option<u32>, Query, description = "Exactly N levels below the anchor"),
    ),
    responses(
        (status = 200, description = "The geofence (GeoJSON in the envelope, or a raw export format)", body = Object),
        (status = 400, description = "`depth` and `level` are mutually exclusive", body = ApiError),
        (status = 404, description = "No such geofence", body = ApiError),
        (status = 409, description = "The name is shared by several modes and no `mode` was given; the message names them", body = ApiError),
    ),
)]
async fn get_one(
    conn: web::Data<KojiDb>,
    path: web::Path<String>,
    query: web::Query<ReadQuery>,
) -> Result<HttpResponse, ServiceError> {
    let id = path.into_inner();
    let return_type = query.return_type(ReturnTypeArg::Feature);
    // Only a name is narrowed by `?mode=`; a numeric id already names one row.
    let mode = query.mode_filter()?;

    let coll = match query.hierarchy()? {
        Some(spec) => {
            let anchor = match id.parse::<u32>() {
                Ok(id) => Anchor::Id(id),
                Err(_) => {
                    let root = geofence::Query::get_by_name(&conn.koji, id.clone(), mode)
                        .await
                        .map_err(ServiceError::not_found_or_db_pick_mode(
                            "geofence",
                            format!("no geofence {id}"),
                        ))?;
                    Anchor::Id(root.id)
                }
            };
            geofence::Query::descendants(&conn.koji, anchor, spec).await?
        }
        None => {
            let geometry = geofence::Query::get_one_koji_in_mode(&conn.koji, id.clone(), mode)
                .await
                .map_err(ServiceError::not_found_or_db_pick_mode(
                    "geofence",
                    format!("no geofence {id}"),
                ))?;
            koji_core::KojiGeometryCollection::new(vec![geometry])
        }
    };

    Ok(respond_geo(coll, return_type))
}

/// `PATCH /api/v2/geofences/{id}` — update a geofence by id → `200` envelope;
/// `404` on a missing id. `dragonite_links` sets or clears the geofence's Area on
/// each named Dragonite webhook; an Area another geofence of the same mode holds
/// on that webhook is a `409`, and a webhook that is not a Dragonite is a `422`.
#[utoipa::path(
    patch,
    path = "/api/v2/geofences/{id}",
    tag = "geofences",
    params(("id" = u32, Path, description = "Geofence id")),
    request_body = PatchGeofence,
    responses(
        (status = 200, description = "Updated geofence record", body = Object),
        (status = 404, description = "No such geofence", body = ApiError),
        (status = 409, description = "A Dragonite Area is held by another geofence of the same mode", body = ApiError),
        (status = 422, description = "`dragonite_links` names a webhook that is missing or not a Dragonite", body = ApiError),
    ),
)]
async fn update(
    conn: web::Data<KojiDb>,
    hub: web::Data<crate::internal::realtime::RealtimeHub>,
    path: web::Path<u32>,
    body: web::Json<PatchGeofence>,
) -> Result<HttpResponse, ServiceError> {
    let id = path.into_inner();
    let link_changes: Vec<LinkChange> = body
        .dragonite_links
        .iter()
        .flatten()
        .map(|change| (*change).into())
        .collect();
    // Fetch existing row — 404 on missing (also supplies the base for the merge).
    let existing = geofence::Query::get_one(&conn.koji, id.to_string())
        .await
        .map_err(ServiceError::not_found_or_db(
            "geofence",
            format!("no geofence {id}"),
        ))?;
    // Build a full JSON from the existing model, then overlay only the fields
    // the PATCH body supplied (skip_serializing_if = "Option::is_none" ensures
    // absent fields are absent from the patch value).
    let mut merged = serde_json::to_value(&existing).map_err(ServiceError::internal)?;
    let body_links_given = body.dragonite_links.is_some();
    let patch_value = serde_json::to_value(body.into_inner()).map_err(ServiceError::internal)?;
    let carries_projects = patch_value.get("projects").is_some();
    super::crud::merge_patch(&mut merged, &patch_value);
    // Links are checked against the mode the geofence ends up in, before
    // anything is written: a mode change alone can make two geofences share an
    // Area on a webhook.
    let mode = merged_mode(&merged);
    let touches_links = body_links_given || mode != existing.mode;
    if touches_links {
        dragonite_link::Query::validate_links(&conn.koji, id, mode, &link_changes).await?;
    }
    // Snapshot pre-mutation project links (only needed when the PATCH body
    // carries `projects` — otherwise membership isn't touched at all).
    let before_pids = if carries_projects {
        koji_db::db::geofence_project::Query::project_ids_for_geofence(&conn.koji, id)
            .await
            .unwrap_or_default()
    } else {
        vec![]
    };
    let mut record = geofence::Query::upsert_json_return(&conn.koji, id, merged).await?;
    // The upsert leaves the Dragonite links alone; the body's links are their own write.
    if touches_links {
        dragonite_link::Query::set_links(&conn.koji, id, mode, &link_changes).await?;
    }
    record["dragonite_links"] = dragonite_link::Query::json_for_geofence(&conn.koji, id).await?;
    for (t, ev) in crate::internal::realtime::topics::updated("geofence", id as i64, record.clone())
    {
        hub.publish(&t, ev);
    }
    emit_geofence_updated(&conn.koji, id as u64, &record).await;
    if carries_projects {
        emit_geofence_membership_diff(&conn.koji, id, &before_pids).await;
    }
    Ok(ApiResponse::success(record))
}

/// The mode the upsert will store for a merged geofence value. It parses the
/// field the way `to_geofence` does (legacy aliases accepted, anything else
/// `Unset`), so the link check and the write agree on the mode.
fn merged_mode(merged: &serde_json::Value) -> Mode {
    koji_db::utils::get_enum(
        merged
            .get("mode")
            .map(|mode| mode.as_str().unwrap_or("unset")),
    )
}

/// Diff a single geofence's project membership before vs after a
/// create/update and emit `project.geofences_changed` (via
/// [`crate::utils::outbox::emit_membership_diff`]) for every affected
/// project. `before_pids` is the pre-mutation link set (empty on create);
/// `after` is resolved fresh from `geofence_project` (source of truth at
/// emit time, mirroring [`emit_geofence_updated`]).
async fn emit_geofence_membership_diff(
    db: &sea_orm::DatabaseConnection,
    geofence_id: u32,
    before_pids: &[u32],
) {
    let after_pids =
        koji_db::db::geofence_project::Query::project_ids_for_geofence(db, geofence_id)
            .await
            .unwrap_or_default();

    let mut before = std::collections::HashMap::new();
    for pid in before_pids {
        before
            .entry(*pid)
            .or_insert_with(std::collections::BTreeSet::new)
            .insert(geofence_id);
    }
    let mut after = std::collections::HashMap::new();
    for pid in &after_pids {
        after
            .entry(*pid)
            .or_insert_with(std::collections::BTreeSet::new)
            .insert(geofence_id);
    }
    crate::utils::outbox::emit_membership_diff(db, &before, &after).await;
}

/// Emit `geofence.updated` to the outbox with the current `projectIds[]`
/// (resolved fresh from `geofence_project`, not from the upsert body — a
/// PATCH may not touch `projects` at all, so the join table is the source of
/// truth at emit time). `record` is the flat `geofence::Model` JSON returned
/// by `upsert_json_return` (top-level `name`, not nested under `properties` —
/// the geofence handlers return the raw row, not a GeoJSON Feature).
async fn emit_geofence_updated(
    db: &sea_orm::DatabaseConnection,
    id: u64,
    record: &serde_json::Value,
) {
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
    super::crud::delete_or_404(result.rows_affected, "geofence")?;
    for (t, ev) in crate::internal::realtime::topics::deleted("geofence", id as i64) {
        hub.publish(&t, ev);
    }
    Ok(HttpResponse::build(StatusCode::NO_CONTENT).finish())
}

/// `POST /api/v2/geofences/link-dragonite` — link geofences to a Dragonite
/// webhook's Areas by name.
///
/// Reads every Area from the Dragonite the `webhook_id` row names and links each
/// to the geofences with exactly its name, in every mode, but only among the
/// geofences that row reaches: all of them for a global row, the project's for a
/// scoped one, none for an inactive one. A geofence already linked to a
/// different Area on this webhook keeps its link and is reported as a conflict;
/// one whose Area is already held by another geofence of its mode on this
/// webhook stays unlinked and is reported as `area_taken`. The answer lists what
/// was linked, what already was, those two, the Areas no geofence is named after
/// and the unlinked geofences no Area is named after. `dry_run` writes nothing
/// and answers the same lists.
#[utoipa::path(
    post,
    path = "/api/v2/geofences/link-dragonite",
    tag = "geofences",
    request_body = LinkDragonite,
    responses(
        (status = 200, description = "The link plan: `linked`, `already_linked`, `conflicts`, `area_taken`, `areas_without_geofence`, `geofences_without_area`", body = Object),
        (status = 404, description = "No such webhook", body = ApiError),
        (status = 422, description = "The webhook is not a Dragonite", body = ApiError),
        (status = 502, description = "Dragonite could not be read", body = ApiError),
    ),
)]
async fn link_dragonite(
    conn: web::Data<KojiDb>,
    body: web::Json<LinkDragonite>,
) -> Result<HttpResponse, ServiceError> {
    let target = dragonite_target(&conn.koji, body.webhook_id).await?;
    let areas = client_for(&target)
        .list_areas()
        .await
        .map_err(|e| ServiceError::Upstream {
            field: "dragonite",
            message: format!("could not read Dragonite areas: {e}"),
        })?;
    // A write pass reads under a lock on the target row, so a concurrent PATCH
    // or link pass cannot change a link between the plan and the write and
    // leave `linked` naming a link that was never stored.
    let plan = if body.dry_run {
        let rows = dragonite_link::Query::link_rows(&conn.koji, &target).await?;
        let taken = dragonite_link::Query::taken_areas(&conn.koji, target.id).await?;
        plan_links(&areas, &rows, &taken)
    } else {
        let txn = conn.koji.begin().await?;
        // Re-read the row locked: a webhook edited or deactivated since the
        // Areas were read reaches what it reaches now.
        let target = dragonite_link::Query::lock_target(&txn, target.id).await?;
        let rows = dragonite_link::Query::link_rows(&txn, &target).await?;
        let taken = dragonite_link::Query::taken_areas(&txn, target.id).await?;
        let plan = plan_links(&areas, &rows, &taken);
        let links: Vec<(u32, u32)> = plan
            .linked
            .iter()
            .map(|l| (l.geofence, l.dragonite_area_id))
            .collect();
        dragonite_link::Query::link_unlinked(&txn, target.id, &links).await?;
        txn.commit().await?;
        plan
    };

    let mut answer = serde_json::to_value(&plan).map_err(ServiceError::internal)?;
    answer["webhook_id"] = json!(target.id);
    answer["dry_run"] = json!(body.dry_run);
    Ok(ApiResponse::success(answer))
}

/// The Dragonite row `webhook_id`: `404` when there is none, `422` when the row
/// is another mode.
pub(super) async fn dragonite_target(
    db: &sea_orm::DatabaseConnection,
    webhook_id: u64,
) -> Result<webhook::Model, ServiceError> {
    let target = webhook::Entity::find_by_id(webhook_id)
        .one(db)
        .await?
        .ok_or_else(|| ServiceError::NotFound {
            field: "webhook",
            message: format!("no webhook {webhook_id}"),
        })?;
    if target.mode != koji_db::db::sea_orm_active_enums::WebhookMode::Dragonite {
        return Err(ServiceError::Unprocessable {
            field: Some("webhook_id".to_string()),
            message: format!("webhook {webhook_id} is not a Dragonite"),
        });
    }
    Ok(target)
}

/// `POST /api/v2/geofences/publish` — publish several geofences, or preview it.
///
/// Plans first: each geofence in `ids` is written to every active Dragonite
/// webhook that reaches it and holds a link for it ([`plan_publish`]), and a
/// geofence that sends nothing is listed in `skipped` with the reason (`not_found`,
/// `no_target`: no webhook reaches it, `no_link`: the ones that do hold no link).
/// `dry_run` answers `200 { dry_run, writes, skipped }` and sends nothing. Without
/// it the same plan is sent, one [`area.geofence_updated`](TOPIC_GEOFENCE_UPDATED)
/// event per write, and the answer is `202` with the same lists, each write
/// carrying its `event_id`. Skipped geofences are not an error.
#[utoipa::path(
    post,
    path = "/api/v2/geofences/publish",
    tag = "geofences",
    request_body = PublishGeofences,
    responses(
        (status = 200, description = "Dry run: `{ dry_run, writes, skipped }`, nothing sent", body = Object),
        (status = 409, description = "`writes` differs from the current plan: nothing sent, the current plan in `plan`", body = Object),
        (status = 202, description = "Events enqueued: `{ dry_run, writes, skipped }`, each write with its `event_id`", body = Object),
    ),
)]
pub(crate) async fn publish_many(
    conn: web::Data<KojiDb>,
    body: web::Json<PublishGeofences>,
) -> Result<HttpResponse, ServiceError> {
    let ids = &body.ids;

    if body.dry_run {
        let (mut plan, found) = plan_for(&conn.koji, ids).await?;
        name_areas(&mut plan, &found).await;
        return Ok(publish_plan_response(StatusCode::OK, true, &plan));
    }

    // Plan and send in one transaction: the plan is read from one snapshot, and
    // a failure part way (a geometry that will not parse, an insert error)
    // rolls back every event queued so far, so a retry never sends twice.
    let txn = conn.koji.begin().await?;
    let (mut plan, found) = plan_for(&txn, ids).await?;
    if let Some(approved) = &body.writes {
        let mut approved: Vec<ApprovedWrite> = approved.clone();
        let mut planned: Vec<ApprovedWrite> = plan
            .writes
            .iter()
            .map(|w| ApprovedWrite {
                geofence: w.geofence,
                webhook_id: w.webhook_id,
                dragonite_area_id: w.dragonite_area_id,
            })
            .collect();
        approved.sort();
        planned.sort();
        if approved != planned {
            return Ok(plan_changed_response(&plan));
        }
    }

    // A batch of fences at a time, so their geometries are not all held at once.
    let mut sendable: Vec<u32> = plan.writes.iter().map(|w| w.geofence).collect();
    sendable.dedup();
    let mut write_at: std::collections::HashMap<(u32, u64), usize> = plan
        .writes
        .iter()
        .enumerate()
        .map(|(i, w)| ((w.geofence, w.webhook_id), i))
        .collect();
    for batch in sendable.chunks(100) {
        let models = geofence::Entity::find()
            .filter(geofence::Column::Id.is_in(batch.iter().copied()))
            .all(&txn)
            .await?;
        // A planned fence that is gone changes the plan: send nothing.
        if models.len() != batch.len() {
            return Ok(plan_changed_response(&plan));
        }
        for model in models {
            let Some(targets) = found.get(&model.id) else {
                return Ok(plan_changed_response(&plan));
            };
            let template = geofence_updated(&model).map_err(ServiceError::internal)?;
            let events = publish_to(
                &txn,
                TOPIC_GEOFENCE_UPDATED,
                &targets.linked,
                |target, link| GeofenceUpdated {
                    webhook_id: target.id,
                    dragonite_area_id: link.dragonite_area_id as i64,
                    ..template.clone()
                },
            )
            .await?;
            for event in events.as_array().into_iter().flatten() {
                let webhook_id = event["webhook_id"].as_u64().unwrap_or_default();
                if let Some(i) = write_at.remove(&(model.id, webhook_id)) {
                    plan.writes[i].event_id = event["event_id"].as_str().map(str::to_string);
                }
            }
        }
    }
    txn.commit().await?;
    Ok(publish_plan_response(StatusCode::ACCEPTED, false, &plan))
}

/// Plan publishing `ids` from the current geofences, links and webhooks, and
/// the targets the plan was made from.
async fn plan_for<C: sea_orm::ConnectionTrait>(
    db: &C,
    ids: &[u32],
) -> Result<
    (
        super::publish_plan::PublishPlan,
        std::collections::HashMap<u32, koji_db::db::dragonite_link::PublishTargets>,
    ),
    ServiceError,
> {
    let geofences = geofence::Query::summaries(db, ids).await?;
    let found = dragonite_link::Query::publish_targets_for(db, ids).await?;
    let plan = plan_publish(ids, &geofences, &found);
    Ok((plan, found))
}

/// `409`: the writes differ from the ones approved. Nothing was sent; `plan`
/// is what a send would write now, for the caller to show again.
fn plan_changed_response(plan: &super::publish_plan::PublishPlan) -> HttpResponse {
    HttpResponse::build(StatusCode::CONFLICT).json(json!({
        "status": "error",
        "error": {
            "code": "plan_changed",
            "message": "the publish plan changed since it was shown; nothing was sent",
            "field": "writes",
        },
        "plan": { "dry_run": false, "writes": plan.writes, "skipped": plan.skipped },
    }))
}

/// How long a dry run waits for one Dragonite to list its Areas.
const AREA_NAMES_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

/// Fill each write's `dragonite_area_name` from the Areas its Dragonite lists.
/// Best effort: a Dragonite that errors or is slow leaves its names `None`, so
/// a preview never fails on an unreachable Dragonite.
async fn name_areas(
    plan: &mut super::publish_plan::PublishPlan,
    found: &std::collections::HashMap<u32, koji_db::db::dragonite_link::PublishTargets>,
) {
    let mut targets: std::collections::BTreeMap<u64, &webhook::Model> = Default::default();
    for write in &plan.writes {
        if let Some(target) = found
            .get(&write.geofence)
            .and_then(|t| t.linked.iter().find(|(t, _)| t.id == write.webhook_id))
        {
            targets.entry(write.webhook_id).or_insert(&target.0);
        }
    }
    let listed = futures_util::future::join_all(targets.values().map(|target| async move {
        let areas = tokio::time::timeout(AREA_NAMES_TIMEOUT, client_for(target).list_areas())
            .await
            .ok()
            .and_then(Result::ok)
            .unwrap_or_default();
        (target.id, areas)
    }))
    .await;
    let names: std::collections::HashMap<(u64, i64), String> = listed
        .into_iter()
        .flat_map(|(webhook_id, areas)| {
            areas
                .into_iter()
                .filter_map(move |a| Some(((webhook_id, a.id?), a.name?)))
        })
        .collect();
    for write in &mut plan.writes {
        write.dragonite_area_name = names
            .get(&(write.webhook_id, i64::from(write.dragonite_area_id)))
            .cloned();
    }
}

fn publish_plan_response(
    status: StatusCode,
    dry_run: bool,
    plan: &super::publish_plan::PublishPlan,
) -> HttpResponse {
    ApiResponse::success_with_status(
        status,
        json!({ "dry_run": dry_run, "writes": plan.writes, "skipped": plan.skipped }),
    )
}

/// `POST /api/v2/geofences/{id}/publish` — publish a geofence's fence to its
/// linked Dragonite areas.
///
/// Replaces the v1 GET-that-mutates `/geofence/push/{id}` and the old direct
/// controller-DB write (P5, architecture §7): instead of writing the golbat DB
/// inline, it appends one [`area.geofence_updated`](TOPIC_GEOFENCE_UPDATED)
/// event per active Dragonite webhook that reaches the geofence and holds a link
/// for it. The dispatcher delivers each to the `DragoniteSubscriber`, which
/// builds its client from the webhook the event names (PATCH `/v2/areas/{id}` on
/// that Dragonite). Returns `202 { events: [{ webhook_id, event_id,
/// dragonite_area_id }] }`.
///
/// Gated on linkage: `422` saying which is missing, no Dragonite webhook that
/// reaches the geofence or no link on the ones that do; an unknown geofence
/// yields `404`.
#[utoipa::path(
    post,
    path = "/api/v2/geofences/{id}/publish",
    tag = "geofences",
    params(("id" = String, Path, description = "Geofence id or name")),
    responses(
        (status = 202, description = "Publish events enqueued: `{ events: [{ webhook_id, event_id, dragonite_area_id }] }`", body = Object),
        (status = 404, description = "No such geofence", body = ApiError),
        (status = 422, description = "No active Dragonite webhook reaches the geofence, or none that does holds a link for it", body = ApiError),
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
        .map_err(ServiceError::not_found_or_db(
            "geofence",
            format!("no geofence {id}"),
        ))?;

    // Linkage gate: only a geofence a Dragonite row reaches and links can be pushed.
    let targets = linked_targets(&conn.koji, model.id, &model.name).await?;

    let template = geofence_updated(&model).map_err(ServiceError::internal)?;
    let events = publish_to(
        &conn.koji,
        TOPIC_GEOFENCE_UPDATED,
        &targets,
        |target, link| GeofenceUpdated {
            webhook_id: target.id,
            dragonite_area_id: link.dragonite_area_id as i64,
            ..template.clone()
        },
    )
    .await?;

    Ok(ApiResponse::success_with_status(
        StatusCode::ACCEPTED,
        json!({
            "geofence": model.id,
            "topic": TOPIC_GEOFENCE_UPDATED,
            "mode": template.mode,
            "events": events,
        }),
    ))
}

/// Build the [`GeofenceUpdated`] payload for a geofence, with `webhook_id` and
/// `dragonite_area_id` left for the caller to set per target. It targets the
/// fence's own mode (`area_mode_for`), so each mode's fence lands in its own
/// Dragonite column and leaves the others untouched.
///
/// The fence geometry travels as a property-less, id-less GeoJSON Feature
/// (Dragonite accepts a Feature wrapping the Polygon/MultiPolygon). This is
/// deliberately NOT the Phase 1 `Feature::from(&KojiGeometry)`, which would
/// attach KojiMeta (`id`/`name`/`mode`/...) as `properties`:
/// `area_geofence_patch` ships the whole `Feature` to Dragonite verbatim, so
/// adding those props would change the external PATCH payload, and the
/// `GeofenceUpdated.geofence` shape is locked.
fn geofence_updated(model: &geofence::Model) -> Result<GeofenceUpdated, serde_json::Error> {
    let geometry = serde_json::from_value::<Geometry>(model.geometry.clone())?;
    Ok(GeofenceUpdated {
        webhook_id: 0,
        dragonite_area_id: 0,
        mode: area_mode_for(&model.mode),
        geofence: Feature {
            bbox: None,
            geometry: Some(geometry),
            id: None,
            properties: None,
            foreign_members: None,
        },
    })
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
        .map_err(ServiceError::not_found_or_db(
            "geofence",
            format!("no geofence {id}"),
        ))?;
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
        // Guarded to POST so other methods fall through to `/{id}`: a geofence
        // named "link-dragonite" stays reachable by name.
        .service(
            web::resource("/link-dragonite")
                .guard(actix_web::guard::Post())
                .route(web::post().to(link_dragonite)),
        )
        // Guarded to POST so other methods fall through to `/{id}`: a geofence
        // named "publish" stays reachable by name.
        .service(
            web::resource("/publish")
                .guard(actix_web::guard::Post())
                .route(web::post().to(publish_many)),
        )
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
        // Guarded to POST so other methods fall through to `/{id}`: a geofence
        // named "link-dragonite" stays reachable by name.
        .service(
            web::resource("/link-dragonite")
                .guard(actix_web::guard::Post())
                .route(web::post().to(link_dragonite)),
        )
        // Guarded to POST so other methods fall through to `/{id}`: a geofence
        // named "publish" stays reachable by name.
        .service(
            web::resource("/publish")
                .guard(actix_web::guard::Post())
                .route(web::post().to(publish_many)),
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
    use koji_db::db::sea_orm_active_enums::Mode;
    use koji_dragonite::AreaMode;
    use serde_json::json;

    #[test]
    fn merged_mode_reads_legacy_aliases_and_unknown_strings_as_the_upsert_does() {
        assert_eq!(
            merged_mode(&json!({"mode": "circle_pokemon"})),
            Mode::Pokemon
        );
        assert_eq!(merged_mode(&json!({"mode": "auto_quest"})), Mode::Quest);
        assert_eq!(merged_mode(&json!({"mode": "nonsense"})), Mode::Unset);
        assert_eq!(merged_mode(&json!({"mode": "fort"})), Mode::Fort);
    }

    fn sample_geometry() -> serde_json::Value {
        json!({
            "type": "Polygon",
            "coordinates": [[[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 0.0]]]
        })
    }

    fn model_with_mode(mode: Mode) -> geofence::Model {
        geofence::Model {
            id: 1,
            name: "Denver".to_string(),
            parent: None,
            created_at: Default::default(),
            updated_at: Default::default(),
            mode,
            geometry: sample_geometry(),
            geo_type: "Polygon".to_string(),
            min_lat: None,
            min_lng: None,
            max_lat: None,
            max_lng: None,
        }
    }

    /// The Dragonite PATCH body a published `mode` fence produces.
    fn published_patch(mode: Mode) -> (AreaMode, serde_json::Value) {
        let payload = geofence_updated(&model_with_mode(mode)).unwrap();
        let patch = koji_dragonite::area_geofence_patch(payload.mode, &payload.geofence);
        (payload.mode, serde_json::to_value(&patch).unwrap())
    }

    #[test]
    fn publish_unset_fence_sets_only_the_base_geofence() {
        let (mode, patch) = published_patch(Mode::Unset);
        assert_eq!(mode, AreaMode::Base);
        assert_eq!(patch["geofence"]["geometry"]["type"], "Polygon");
        assert_eq!(
            patch.as_object().unwrap().len(),
            1,
            "only `geofence`: {patch}"
        );
    }

    #[test]
    fn publish_pokemon_fence_sets_only_pokemon_mode_fence() {
        let (mode, patch) = published_patch(Mode::Pokemon);
        assert_eq!(mode, AreaMode::Pokemon);
        assert_eq!(
            patch["pokemon_mode"]["geofence"]["geometry"]["type"],
            "Polygon"
        );
        assert_eq!(
            patch.as_object().unwrap().len(),
            1,
            "only `pokemon_mode`: {patch}"
        );
        assert!(
            patch.get("geofence").is_none(),
            "base fence untouched: {patch}"
        );
    }

    #[test]
    fn publish_quest_fence_sets_only_quest_mode_fence() {
        let (mode, patch) = published_patch(Mode::Quest);
        assert_eq!(mode, AreaMode::Quest);
        assert_eq!(
            patch["quest_mode"]["geofence"]["geometry"]["type"],
            "Polygon"
        );
        assert_eq!(
            patch.as_object().unwrap().len(),
            1,
            "only `quest_mode`: {patch}"
        );
    }

    #[test]
    fn publish_fort_fence_sets_only_fort_mode_fence() {
        let (mode, patch) = published_patch(Mode::Fort);
        assert_eq!(mode, AreaMode::Fort);
        assert_eq!(
            patch["fort_mode"]["geofence"]["geometry"]["type"],
            "Polygon"
        );
        assert_eq!(
            patch.as_object().unwrap().len(),
            1,
            "only `fort_mode`: {patch}"
        );
    }

    #[test]
    fn fences_of_different_modes_on_one_area_patch_disjoint_columns() {
        let (_, base) = published_patch(Mode::Unset);
        let (_, pokemon) = published_patch(Mode::Pokemon);
        let columns = |v: &serde_json::Value| -> Vec<String> {
            v.as_object().unwrap().keys().cloned().collect()
        };
        assert_eq!(columns(&base), ["geofence"]);
        assert_eq!(columns(&pokemon), ["pokemon_mode"]);
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
        assert!(dto.dragonite_links.is_none());
    }

    #[test]
    fn patch_geofence_tells_a_link_from_an_unlink_from_no_change() {
        let links = |v: serde_json::Value| {
            serde_json::from_value::<PatchGeofence>(v)
                .unwrap()
                .dragonite_links
                .map(|l| {
                    l.into_iter()
                        .map(|c| (c.webhook_id, c.dragonite_area_id))
                        .collect::<Vec<_>>()
                })
        };
        assert_eq!(
            links(json!({ "dragonite_links": [
                { "webhook_id": 3, "dragonite_area_id": 12 },
                { "webhook_id": 4, "dragonite_area_id": null },
            ] })),
            Some(vec![(3, Some(12)), (4, None)])
        );
        assert_eq!(links(json!({ "dragonite_links": [] })), Some(vec![]));
        assert_eq!(links(json!({ "name": "x" })), None);
    }

    #[test]
    fn a_link_change_must_name_its_area_or_null() {
        let parse = |v: serde_json::Value| serde_json::from_value::<PatchGeofence>(v);
        // A missing key is not a silent unlink.
        assert!(parse(json!({ "dragonite_links": [{ "webhook_id": 3 }] })).is_err());
        assert!(
            parse(json!({ "dragonite_links": [{ "webhook_id": 3, "dragonite_area_id": -1 }] }))
                .is_err()
        );
        assert!(parse(json!({ "dragonite_links": [{ "dragonite_area_id": 1 }] })).is_err());
    }

    #[test]
    fn patch_geofence_never_serializes_links_into_the_upsert_value() {
        let dto: PatchGeofence = serde_json::from_value(json!({
            "dragonite_links": [{ "webhook_id": 3, "dragonite_area_id": 12 }]
        }))
        .unwrap();
        assert!(
            serde_json::to_value(&dto)
                .unwrap()
                .get("dragonite_links")
                .is_none()
        );
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
        assert!(
            feature["properties"]
                .get("geometry")
                .is_none_or(|g| g.is_null())
        );
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

    // ── ids/bbox query parsing ────────────────────────────────────────────

    #[test]
    fn read_query_ids_none_when_absent() {
        let q = ReadQuery::default();
        assert_eq!(q.ids().unwrap(), None);
    }

    #[test]
    fn read_query_ids_splits_trims_and_ignores_blanks() {
        let q = ReadQuery {
            ids: Some(" 1, 2 ,,3".to_string()),
            ..Default::default()
        };
        assert_eq!(q.ids().unwrap(), Some(vec![1, 2, 3]));
    }

    #[test]
    fn read_query_ids_non_numeric_segment_is_400() {
        let q = ReadQuery {
            ids: Some("1,foo".to_string()),
            ..Default::default()
        };
        let err = q.ids().unwrap_err();
        assert!(
            matches!(err, ServiceError::Invalid { field, .. } if field.as_deref() == Some("ids"))
        );
    }

    #[test]
    fn read_query_bbox_none_when_absent() {
        let q = ReadQuery::default();
        assert_eq!(q.bbox().unwrap(), None);
    }

    #[test]
    fn read_query_bbox_parses_four_numbers() {
        let q = ReadQuery {
            bbox: Some("-1.5,2,3,4.25".to_string()),
            ..Default::default()
        };
        assert_eq!(q.bbox().unwrap(), Some([-1.5, 2.0, 3.0, 4.25]));
    }

    #[test]
    fn read_query_bbox_wrong_count_is_400() {
        let q = ReadQuery {
            bbox: Some("1,2,3".to_string()),
            ..Default::default()
        };
        let err = q.bbox().unwrap_err();
        assert!(
            matches!(err, ServiceError::Invalid { field, .. } if field.as_deref() == Some("bbox"))
        );
    }

    #[test]
    fn read_query_bbox_non_numeric_is_400() {
        let q = ReadQuery {
            bbox: Some("1,2,3,nope".to_string()),
            ..Default::default()
        };
        let err = q.bbox().unwrap_err();
        assert!(
            matches!(err, ServiceError::Invalid { field, .. } if field.as_deref() == Some("bbox"))
        );
    }

    #[test]
    fn read_query_mode_filter_parses_and_rejects() {
        let q = ReadQuery {
            mode: Some("pokemon".into()),
            ..Default::default()
        };
        assert_eq!(
            q.mode_filter().unwrap(),
            Some(koji_db::db::sea_orm_active_enums::Mode::Pokemon)
        );
        let none = ReadQuery::default();
        assert_eq!(none.mode_filter().unwrap(), None);
        let bad = ReadQuery {
            mode: Some("raid".into()),
            ..Default::default()
        };
        assert!(bad.mode_filter().is_err());
    }

    #[test]
    fn read_query_projects_parses_csv_and_rejects_garbage() {
        let q = ReadQuery {
            projects: Some("1, 2,3".into()),
            ..Default::default()
        };
        assert_eq!(q.project_ids().unwrap(), Some(vec![1, 2, 3]));
        let bad = ReadQuery {
            projects: Some("1,x".into()),
            ..Default::default()
        };
        assert!(bad.project_ids().is_err());
    }

    #[test]
    fn read_query_bbox_non_finite_is_400() {
        // `nan`/`inf` parse fine as f64, but would make bbox_overlaps always
        // false — silently disabling the filter instead of erroring. Every
        // non-finite slot (not just the first) must be rejected.
        for raw in [
            "nan,nan,nan,nan",
            "0,0,10,inf",
            "-inf,0,10,10",
            "0,NaN,10,10",
        ] {
            let q = ReadQuery {
                bbox: Some(raw.to_string()),
                ..Default::default()
            };
            let err = q.bbox().unwrap_err();
            assert!(
                matches!(err, ServiceError::Invalid { field, .. } if field.as_deref() == Some("bbox")),
                "bbox={raw:?} must be rejected as Invalid"
            );
        }
    }

    // ── features_intersecting_bbox (pure — no DB) ───────────────────────────

    /// A `Point` feature at `(lon, lat)`, id set so tests can tell which
    /// features survived the filter.
    fn point_feature(id: u64, lon: f64, lat: f64) -> Feature {
        Feature {
            bbox: None,
            geometry: Some(Geometry::new(geojson::GeometryValue::Point {
                coordinates: geojson::Position::from([lon, lat]),
            })),
            id: Some(geojson::feature::Id::Number(id.into())),
            properties: None,
            foreign_members: None,
        }
    }

    /// A `Polygon` feature whose bbox is exactly `[min_lon, min_lat, max_lon,
    /// max_lat]` (an axis-aligned rectangle ring).
    fn rect_feature(id: u64, min_lon: f64, min_lat: f64, max_lon: f64, max_lat: f64) -> Feature {
        let ring = vec![
            geojson::Position::from([min_lon, min_lat]),
            geojson::Position::from([max_lon, min_lat]),
            geojson::Position::from([max_lon, max_lat]),
            geojson::Position::from([min_lon, max_lat]),
            geojson::Position::from([min_lon, min_lat]),
        ];
        Feature {
            bbox: None,
            geometry: Some(Geometry::new(geojson::GeometryValue::Polygon {
                coordinates: vec![ring],
            })),
            id: Some(geojson::feature::Id::Number(id.into())),
            properties: None,
            foreign_members: None,
        }
    }

    fn feature_ids(fc: &FeatureCollection) -> Vec<u64> {
        fc.features
            .iter()
            .map(|f| match &f.id {
                Some(geojson::feature::Id::Number(n)) => n.as_u64().unwrap(),
                _ => panic!("expected numeric id"),
            })
            .collect()
    }

    #[test]
    fn features_intersecting_bbox_keeps_only_the_overlapping_feature() {
        let inside = rect_feature(1, 2.0, 2.0, 3.0, 3.0); // fully within the box
        let far_outside = rect_feature(2, 100.0, 100.0, 101.0, 101.0);
        let fc = FeatureCollection {
            bbox: None,
            features: vec![inside, far_outside],
            foreign_members: None,
        };

        let filtered = features_intersecting_bbox(fc, [0.0, 0.0, 10.0, 10.0]);

        assert_eq!(feature_ids(&filtered), vec![1]);
    }

    #[test]
    fn features_intersecting_bbox_keeps_boundary_straddling_feature() {
        // A point sitting exactly on the query box's corner: not "inside" by
        // any margin, but AABB overlap must still count it (non-strict `<`/`>`).
        let on_the_corner = point_feature(1, 10.0, 10.0);
        let fc = FeatureCollection {
            bbox: None,
            features: vec![on_the_corner],
            foreign_members: None,
        };

        let filtered = features_intersecting_bbox(fc, [0.0, 0.0, 10.0, 10.0]);

        assert_eq!(feature_ids(&filtered), vec![1]);
    }

    #[test]
    fn features_intersecting_bbox_keeps_straddling_polygon() {
        // A real (non-degenerate) polygon whose bbox is partially inside and
        // partially outside the query box — most of it sits outside [0,0,10,10],
        // but it overlaps the box's right edge. It must survive the filter, and
        // a polygon fully outside must not.
        let straddler = rect_feature(1, 8.0, 8.0, 20.0, 20.0);
        let fully_outside = rect_feature(2, 11.0, 11.0, 20.0, 20.0);
        let fc = FeatureCollection {
            bbox: None,
            features: vec![straddler, fully_outside],
            foreign_members: None,
        };

        let filtered = features_intersecting_bbox(fc, [0.0, 0.0, 10.0, 10.0]);

        assert_eq!(feature_ids(&filtered), vec![1]);
    }

    #[test]
    fn features_intersecting_bbox_drops_feature_with_no_geometry() {
        let no_geom = Feature {
            bbox: None,
            geometry: None,
            id: Some(geojson::feature::Id::Number(1.into())),
            properties: None,
            foreign_members: None,
        };
        let fc = FeatureCollection {
            bbox: None,
            features: vec![no_geom],
            foreign_members: None,
        };

        let filtered = features_intersecting_bbox(fc, [0.0, 0.0, 10.0, 10.0]);

        assert!(filtered.features.is_empty());
    }

    #[test]
    fn bbox_overlaps_matches_standard_aabb_rules() {
        // Fully separated on the x axis.
        assert!(!bbox_overlaps([0.0, 0.0, 1.0, 1.0], [2.0, 0.0, 3.0, 1.0]));
        // Touching at a single edge counts as overlap.
        assert!(bbox_overlaps([0.0, 0.0, 1.0, 1.0], [1.0, 0.0, 2.0, 1.0]));
        // Fully contained.
        assert!(bbox_overlaps([0.0, 0.0, 10.0, 10.0], [2.0, 2.0, 3.0, 3.0]));
    }
}
