//! v2 golbat-data fetch — two complementary surfaces over the same koji-golbat
//! query path (architecture §4.5, §6), both returning the golbat points of a
//! category (`gym|pokestop|spawnpoint|station|fort`):
//!
//! - `GET /api/v2/geofences/{id}/golbat-data` — the **saved-fence** convenience:
//!   the area is the path `{id}` geofence (by id or name), resolved via
//!   [`utils::create_or_find_collection`]; a missing geofence ⇒ `404`. Mounted
//!   under [`super::geofences::scope()`] as the `/{id}/golbat-data` sub-resource.
//! - `POST /api/v2/golbat-data/{category}` (+ `/stats`) — the **arbitrary-area**
//!   surface: the drawn area / bbox rides the request body (a GET can't carry a
//!   polygon). Ports the v1 `/internal/data/area`(+`bound`) and `/area_stats`
//!   handlers. Mounted via [`scope()`]. Category in the path (matches the old
//!   `/internal/data/area/{category}`).
//!
//! Both reuse the same `koji-golbat` area query ([`utils::points_from_area`]) and
//! the up-front [`VALID_CATEGORIES`] allow-list, so an unknown category is a clean
//! `400` before any DB work.

use actix_web::{HttpResponse, post, web};
use koji_core::Precision;
use geojson::{Feature, FeatureCollection, Geometry, GeometryValue};
use koji_core::{KojiBbox, SpawnpointTth};
use koji_db::KojiDb;
use koji_golbat::GenericDataToVec;
use koji_golbat::entities::{gym, pokestop, spawnpoint, station};
use serde::Deserialize;
use serde_json::json;
use utoipa::{IntoParams, ToSchema};

use crate::utils::api_response::{ApiError, ApiResponse};
use crate::utils::error::ServiceError;
use crate::utils::{self};

/// The golbat-data categories the koji-golbat query path understands. Kept in
/// sync with the `match category` arms in [`utils::points_from_area`]; validating
/// up-front lets an unknown category surface as a clean `400` before any DB work.
const VALID_CATEGORIES: [&str; 5] = ["gym", "pokestop", "spawnpoint", "station", "fort"];

/// Query for `GET /api/v2/geofences/{id}/golbat-data`. `pub(crate)` because the
/// handler is mounted cross-module (from [`super::geofences::scope()`]), so its
/// `web::Query<GolbatDataQuery>` arg type must be at least as visible as the
/// handler the route names.
#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct GolbatDataQuery {
    /// Golbat data category (`gym|pokestop|spawnpoint|station|fort`).
    category: String,
    /// Only points updated within the last N seconds (`0` = no filter).
    /// camelCase wire (a config-style query param), defaulting to `0`.
    #[serde(rename = "lastSeen", default)]
    last_seen: u32,
}

/// `GET /api/v2/geofences/{id}/golbat-data?category=&lastSeen=` — the category's
/// golbat points within the path `{id}` geofence (by id or name), wrapped in the
/// v2 [`ApiResponse`] envelope.
///
/// Mounted under [`super::geofences::scope()`] alongside `/{id}` and
/// `/{id}/publish`, so the geofence id/name comes from the path. An unknown
/// `category` ⇒ `400`; a missing geofence ⇒ `404`.
#[utoipa::path(
    get,
    path = "/api/v2/geofences/{id}/golbat-data",
    tag = "golbat-data",
    params(
        ("id" = String, Path, description = "Geofence id or name"),
        GolbatDataQuery,
    ),
    responses(
        (status = 200, description = "The category's golbat points within the geofence", body = Object),
        (status = 400, description = "Unknown category", body = ApiError),
        (status = 404, description = "No such geofence", body = ApiError),
    ),
)]
// `ServiceError` is intentionally large (carries `DbErr`/`ModelError` by value) —
// matches the crate-wide allow in `utils::error`.
#[allow(clippy::result_large_err)]
pub(crate) async fn golbat_data(
    conn: web::Data<KojiDb>,
    path: web::Path<String>,
    query: web::Query<GolbatDataQuery>,
) -> Result<HttpResponse, ServiceError> {
    let instance = path.into_inner();
    let query = query.into_inner();
    let category = query.category;
    let last_seen = query.last_seen;

    // Validate the category up-front so an unknown one is a clean 400 (rather than
    // the generic 500 the downstream `DbErr::Custom("Invalid Category")` would
    // produce). Preserves the v1 category set.
    if !VALID_CATEGORIES.contains(&category.as_str()) {
        return Err(ServiceError::Invalid {
            field: Some("category".to_string()),
            message: format!(
                "unknown category `{category}` (expected one of {})",
                VALID_CATEGORIES.join(", ")
            ),
        });
    }

    // Resolve the geofence's area by the path id/name. `create_or_find_collection`
    // with no data_points / empty area / no parent loads the stored geofence; a
    // missing one yields a `ModelError`, mapped to a 404.
    let area = utils::create_or_find_collection(
        &instance,
        &conn,
        FeatureCollection::default(),
        &None,
        &vec![],
    )
    .await
    .map_err(|_| ServiceError::NotFound {
        field: "geofence",
        message: format!("no geofence {instance}"),
    })?;

    let points = utils::points_from_area(&area, &category, &conn, last_seen, SpawnpointTth::All)
        .await
        .map_err(ServiceError::internal)?;

    Ok(ApiResponse::success(points))
}

// ---------------------------------------------------------------------------
// Arbitrary-area surface: `POST /api/v2/golbat-data/{category}` (+ `/stats`).
// Ports v1 `/internal/data/area`(+`bound`) and `/internal/data/area_stats`.
// ---------------------------------------------------------------------------

/// Body for the arbitrary-area golbat-data POSTs. The drawn area rides the body
/// (a GET can't carry a polygon) as either a `area` geojson container OR a flat
/// `bbox`; `lastSeen`/`tth` mirror the v1 golbat filters. All fields optional —
/// an empty body resolves to an empty area (no points), matching v1's
/// empty-`area` behavior.
///
/// `area` + `bbox` collapse onto ONE downstream path: a `bbox`-only request is
/// turned into a one-Polygon `FeatureCollection`, so both feed the same
/// [`utils::points_from_area`] / per-category `stats` query (which also gives the
/// `bbox` path the `fort` aggregate the v1 `/bound` handler lacked). `area` wins
/// if both are supplied.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AreaReq {
    /// The drawn area as a geojson container (`FeatureCollection`/`Feature`/
    /// `Geometry`). Takes precedence over `bbox`.
    #[schema(value_type = Option<Object>)]
    area: Option<crate::requests::GeoInput>,
    /// A flat lat/lon bounding box (`minLat`/`minLon`/`maxLat`/`maxLon`) — used
    /// only when `area` is absent. Ports the v1 `/internal/data/bound` input.
    bbox: Option<BboxInput>,
    /// Only points updated within the last N seconds (`0` = no filter).
    #[serde(default)]
    last_seen: u32,
    /// Spawnpoint confirmed/unconfirmed filter (`spawnpoint` category only).
    #[serde(default = "default_tth")]
    #[schema(value_type = String)]
    tth: SpawnpointTth,
}

fn default_tth() -> SpawnpointTth {
    SpawnpointTth::All
}

// `SpawnpointTth` has no `Default`, so `AreaReq`'s is hand-written (matching the
// `default_tth` serde default) rather than derived.
impl Default for AreaReq {
    fn default() -> Self {
        Self {
            area: None,
            bbox: None,
            last_seen: 0,
            tth: default_tth(),
        }
    }
}

/// camelCase wire bbox for the v2 body (`{minLat,minLon,maxLat,maxLon}`). The
/// domain [`KojiBbox`] deserializes snake_case (the v1 `BoundsArg` wire), so this
/// thin DTO keeps the v2 body uniformly camelCase (like `lastSeen`); it converts
/// straight into `KojiBbox` for the query path.
#[derive(Debug, Clone, Copy, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BboxInput {
    min_lat: Precision,
    min_lon: Precision,
    max_lat: Precision,
    max_lon: Precision,
}

impl From<BboxInput> for KojiBbox {
    fn from(b: BboxInput) -> Self {
        KojiBbox {
            min_lat: b.min_lat,
            min_lon: b.min_lon,
            max_lat: b.max_lat,
            max_lon: b.max_lon,
        }
    }
}

/// Turn a [`KojiBbox`] into a single-Polygon `FeatureCollection` so a `bbox`-only
/// request flows through the same `area` query path. geojson rings are `[lon,
/// lat]`, wound CCW and closed.
fn bbox_collection(bbox: KojiBbox) -> FeatureCollection {
    let ring = vec![
        geojson::Position::from([bbox.min_lon, bbox.min_lat]),
        geojson::Position::from([bbox.max_lon, bbox.min_lat]),
        geojson::Position::from([bbox.max_lon, bbox.max_lat]),
        geojson::Position::from([bbox.min_lon, bbox.max_lat]),
        geojson::Position::from([bbox.min_lon, bbox.min_lat]),
    ];
    let feature = Feature {
        bbox: None,
        geometry: Some(Geometry::new(GeometryValue::Polygon { coordinates: vec![ring] })),
        id: None,
        properties: None,
        foreign_members: None,
    };
    FeatureCollection {
        bbox: None,
        features: vec![feature],
        foreign_members: None,
    }
}

/// Resolve the request body's `area`/`bbox` into the algorithm-edge
/// `FeatureCollection` the golbat query consumes: `area` if present, else a
/// polygon built from `bbox`, else empty.
fn resolve_area(req: &AreaReq) -> FeatureCollection {
    if req.area.is_some() {
        crate::requests::area_collection(&req.area)
    } else if let Some(bbox) = req.bbox {
        bbox_collection(bbox.into())
    } else {
        FeatureCollection::default()
    }
}

/// Reject an unknown `{category}` up-front as a clean `400` (before any DB work),
/// mirroring [`golbat_data`]'s guard and the v1 category set.
#[allow(clippy::result_large_err)]
fn validate_category(category: &str) -> Result<(), ServiceError> {
    if VALID_CATEGORIES.contains(&category) {
        Ok(())
    } else {
        Err(ServiceError::Invalid {
            field: Some("category".to_string()),
            message: format!(
                "unknown category `{category}` (expected one of {})",
                VALID_CATEGORIES.join(", ")
            ),
        })
    }
}

/// `POST /api/v2/golbat-data/{category}` — the category's golbat points within
/// the body's drawn `area` (or `bbox`), as a `SingleVec` of `[lat, lon]` pairs.
/// Ports v1 `/internal/data/area`(+`bound`); unknown `category` ⇒ `400`.
#[utoipa::path(
    post,
    path = "/api/v2/golbat-data/{category}",
    tag = "golbat-data",
    params(("category" = String, Path, description = "Golbat data category (`gym|pokestop|spawnpoint|station|fort`)")),
    request_body = AreaReq,
    responses(
        (status = 200, description = "The category's golbat points: `{ \"points\": [[lat, lon], …] }`", body = Object),
        (status = 400, description = "Unknown category", body = ApiError),
    ),
)]
#[allow(clippy::result_large_err)]
#[post("/{category}")]
pub(crate) async fn by_area(
    conn: web::Data<KojiDb>,
    path: web::Path<String>,
    payload: web::Json<AreaReq>,
) -> Result<HttpResponse, ServiceError> {
    let category = path.into_inner();
    validate_category(&category)?;
    let req = payload.into_inner();
    let area = resolve_area(&req);

    let points = utils::points_from_area(&area, &category, &conn, req.last_seen, req.tth)
        .await
        .map_err(ServiceError::internal)?
        .to_single_vec();

    log::info!("[DATA-AREA] Returning {} {category}s", points.len());
    Ok(ApiResponse::success(json!({ "points": points })))
}

/// `POST /api/v2/golbat-data/{category}/stats` — the count of the category's
/// golbat points within the body's drawn `area` (or `bbox`). Ports v1
/// `/internal/data/area_stats`; returns `{ "total": <usize> }`.
#[utoipa::path(
    post,
    path = "/api/v2/golbat-data/{category}/stats",
    tag = "golbat-data",
    params(("category" = String, Path, description = "Golbat data category (`gym|pokestop|spawnpoint|station|fort`)")),
    request_body = AreaReq,
    responses(
        (status = 200, description = "Count of the category's golbat points: `{ \"total\": <usize> }`", body = Object),
        (status = 400, description = "Unknown category", body = ApiError),
    ),
)]
#[allow(clippy::result_large_err)]
#[post("/{category}/stats")]
pub(crate) async fn area_stats(
    conn: web::Data<KojiDb>,
    path: web::Path<String>,
    payload: web::Json<AreaReq>,
) -> Result<HttpResponse, ServiceError> {
    let category = path.into_inner();
    validate_category(&category)?;
    let req = payload.into_inner();
    let area = resolve_area(&req);
    let last_seen = req.last_seen;

    // Per-category stat query (mirrors v1 `area_stats`). `fort` has no single
    // stats query, so aggregate its members' totals — consistent with how
    // `points_from_area` treats `fort`.
    let total = match category.as_str() {
        "gym" => {
            gym::Query::stats(&conn.golbat, &area, last_seen)
                .await?
                .total
        }
        "pokestop" => {
            pokestop::Query::stats(&conn.golbat, &area, last_seen)
                .await?
                .total
        }
        "station" => {
            station::Query::stats(&conn.golbat, &area, last_seen)
                .await?
                .total
        }
        "spawnpoint" => {
            spawnpoint::Query::stats(&conn.golbat, &area, last_seen, req.tth)
                .await?
                .total
        }
        "fort" => {
            let gyms = gym::Query::stats(&conn.golbat, &area, last_seen)
                .await?
                .total;
            let pokestops = pokestop::Query::stats(&conn.golbat, &area, last_seen)
                .await?
                .total;
            let stations = station::Query::stats(&conn.golbat, &area, last_seen)
                .await?
                .total;
            gyms + pokestops + stations
        }
        // Unreachable after `validate_category`, but keep the surface total.
        _ => 0,
    };

    log::info!("[DATA-AREA] {category} total: {total}");
    Ok(ApiResponse::success(json!({ "total": total })))
}

/// The `/golbat-data` scope: the arbitrary-area markers + stats POSTs. Mounted
/// into `/api/v2` by [`crate::start`]. (The saved-fence GET lives under
/// [`super::geofences::scope()`], not here.)
pub(crate) fn scope() -> actix_web::Scope {
    web::scope("/golbat-data")
        .service(by_area)
        .service(area_stats)
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::web::Query;

    #[test]
    fn query_defaults_last_seen_to_zero() {
        // Exercises the exact actix query-deser path the handler relies on.
        let q = Query::<GolbatDataQuery>::from_query("category=pokestop").unwrap();
        assert_eq!(q.category, "pokestop");
        assert_eq!(q.last_seen, 0);
    }

    #[test]
    fn query_reads_camelcase_last_seen() {
        let q = Query::<GolbatDataQuery>::from_query("category=gym&lastSeen=3600").unwrap();
        assert_eq!(q.category, "gym");
        assert_eq!(q.last_seen, 3600);
    }

    #[test]
    fn query_requires_category() {
        // `category` is required (not Option) — a query missing it fails to deser.
        assert!(Query::<GolbatDataQuery>::from_query("lastSeen=10").is_err());
    }

    #[test]
    fn valid_categories_match_points_from_area_arms() {
        // Guard: the up-front allow-list mirrors the koji-golbat query arms.
        assert!(VALID_CATEGORIES.contains(&"gym"));
        assert!(VALID_CATEGORIES.contains(&"pokestop"));
        assert!(VALID_CATEGORIES.contains(&"spawnpoint"));
        assert!(VALID_CATEGORIES.contains(&"station"));
        assert!(VALID_CATEGORIES.contains(&"fort"));
        assert!(!VALID_CATEGORIES.contains(&"bogus"));
    }

    // --- arbitrary-area POST surface (T2) ---

    #[test]
    fn validate_category_rejects_unknown_as_invalid_400() {
        // Known categories pass.
        assert!(validate_category("gym").is_ok());
        assert!(validate_category("fort").is_ok());
        // Unknown ⇒ ServiceError::Invalid on field "category" (a clean 400).
        match validate_category("bogus") {
            Err(ServiceError::Invalid { field, message }) => {
                assert_eq!(field.as_deref(), Some("category"));
                assert!(message.contains("bogus"));
            }
            other => panic!("expected Invalid(category), got {other:?}"),
        }
    }

    #[test]
    fn area_req_deserializes_camelcase_and_defaults() {
        // Empty body: no area/bbox, lastSeen 0, tth All.
        let req: AreaReq = serde_json::from_str("{}").unwrap();
        assert!(req.area.is_none() && req.bbox.is_none());
        assert_eq!(req.last_seen, 0);
        assert!(matches!(req.tth, SpawnpointTth::All));

        // camelCase wire: lastSeen + a flat camelCase bbox.
        let req: AreaReq = serde_json::from_str(
            r#"{"bbox":{"minLat":1.0,"minLon":2.0,"maxLat":3.0,"maxLon":4.0},"lastSeen":600,"tth":"Known"}"#,
        )
        .unwrap();
        let bbox: KojiBbox = req.bbox.expect("bbox parsed").into();
        assert_eq!(
            (bbox.min_lat, bbox.min_lon, bbox.max_lat, bbox.max_lon),
            (1.0, 2.0, 3.0, 4.0)
        );
        assert_eq!(req.last_seen, 600);
        assert!(matches!(req.tth, SpawnpointTth::Known));
    }

    #[test]
    fn bbox_collection_builds_one_closed_lonlat_polygon() {
        let bbox = KojiBbox {
            min_lat: 1.0,
            min_lon: 2.0,
            max_lat: 3.0,
            max_lon: 4.0,
        };
        let fc = bbox_collection(bbox);
        assert_eq!(fc.features.len(), 1);
        let geom = fc.features[0].geometry.as_ref().unwrap();
        match &geom.value {
            GeometryValue::Polygon { coordinates: rings } => {
                assert_eq!(rings.len(), 1);
                let ring = &rings[0];
                // 5 coords, closed; geojson order is [lon, lat].
                assert_eq!(ring.len(), 5);
                assert_eq!(ring.first(), ring.last());
                assert_eq!(ring[0], geojson::Position::from([2.0, 1.0])); // [min_lon, min_lat]
                assert_eq!(ring[2], geojson::Position::from([4.0, 3.0])); // [max_lon, max_lat]
            }
            other => panic!("expected Polygon, got {other:?}"),
        }
    }

    #[test]
    fn resolve_area_prefers_area_then_bbox_then_empty() {
        // Neither → empty collection.
        let empty = AreaReq::default();
        assert!(resolve_area(&empty).features.is_empty());

        // bbox-only → one polygon (via bbox_collection).
        let bbox_only: AreaReq = serde_json::from_str(
            r#"{"bbox":{"minLat":0.0,"minLon":0.0,"maxLat":1.0,"maxLon":1.0}}"#,
        )
        .unwrap();
        assert_eq!(resolve_area(&bbox_only).features.len(), 1);

        // area present → area wins (bbox ignored). A FeatureCollection area with
        // one polygon feature resolves to one feature.
        let area_wins: AreaReq = serde_json::from_str(
            r#"{"area":{"type":"FeatureCollection","features":[{"type":"Feature","properties":{},
              "geometry":{"type":"Polygon","coordinates":[[[0.0,0.0],[1.0,0.0],[1.0,1.0],[0.0,1.0],[0.0,0.0]]]}}]},
              "bbox":{"minLat":9.0,"minLon":9.0,"maxLat":9.0,"maxLon":9.0}}"#,
        )
        .unwrap();
        assert_eq!(resolve_area(&area_wins).features.len(), 1);
    }
}
