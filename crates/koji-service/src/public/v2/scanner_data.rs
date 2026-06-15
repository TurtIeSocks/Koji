//! v2 scanner-data fetch — `GET /api/v2/geofences/{id}/scanner-data` (architecture
//! §4.5, §6). Returns the scanner points of a category (`gym|pokestop|spawnpoint|
//! station|fort`) within a geofence, reusing the same koji-scanner query path as
//! the v1 `/internal/data/area` handler.
//!
//! **Area input:** the geofence is the path `{id}` (its id **or** name) of the
//! enclosing `/geofences/{id}` resource — scanner-data is an honest sub-resource
//! of the geofence, not a flat endpoint keyed by an `?instance=` query. The area
//! is resolved via [`utils::create_or_find_collection`] (which reads the stored
//! geofence by id/name); a missing geofence ⇒ `404`. (The v1 handler took a POST
//! body with an `Args` area; a GET can't, so v2 keys off the path geofence.
//! Ad-hoc bbox queries remain on the v1 `/internal/data/bound` endpoint.)
//!
//! Re-keyed here (T3); the route is remounted under `geofences::scope()` in T4.
//! Between the two the handler is unreferenced, so the dead-code lint is silenced
//! crate-wide for this module until T4 wires it.
#![allow(dead_code)]

use actix_web::{HttpResponse, web};
use geojson::FeatureCollection;
use koji_core::SpawnpointTth;
use koji_db::KojiDb;
use serde::Deserialize;

use crate::utils::api_response::ApiResponse;
use crate::utils::error::ServiceError;
use crate::utils::{self};

/// The scanner-data categories the koji-scanner query path understands. Kept in
/// sync with the `match category` arms in [`utils::points_from_area`]; validating
/// up-front lets an unknown category surface as a clean `400` before any DB work.
const VALID_CATEGORIES: [&str; 5] = ["gym", "pokestop", "spawnpoint", "station", "fort"];

/// Query for `GET /api/v2/geofences/{id}/scanner-data`.
#[derive(Debug, Deserialize)]
struct ScannerDataQuery {
    /// Scanner data category (`gym|pokestop|spawnpoint|station|fort`).
    category: String,
    /// Only points updated within the last N seconds (`0` = no filter).
    /// camelCase wire (a config-style query param), defaulting to `0`.
    #[serde(rename = "lastSeen", default)]
    last_seen: u32,
}

/// `GET /api/v2/geofences/{id}/scanner-data?category=&lastSeen=` — the category's
/// scanner points within the path `{id}` geofence (by id or name), wrapped in the
/// v2 [`ApiResponse`] envelope.
///
/// Mounted under [`super::geofences::scope()`] alongside `/{id}` and
/// `/{id}/publish`, so the geofence id/name comes from the path. An unknown
/// `category` ⇒ `400`; a missing geofence ⇒ `404`.
// `ServiceError` is intentionally large (carries `DbErr`/`ModelError` by value) —
// matches the crate-wide allow in `utils::error`.
#[allow(clippy::result_large_err)]
pub(crate) async fn scanner_data(
    conn: web::Data<KojiDb>,
    path: web::Path<String>,
    query: web::Query<ScannerDataQuery>,
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

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::web::Query;

    #[test]
    fn query_defaults_last_seen_to_zero() {
        // Exercises the exact actix query-deser path the handler relies on.
        let q = Query::<ScannerDataQuery>::from_query("category=pokestop").unwrap();
        assert_eq!(q.category, "pokestop");
        assert_eq!(q.last_seen, 0);
    }

    #[test]
    fn query_reads_camelcase_last_seen() {
        let q = Query::<ScannerDataQuery>::from_query("category=gym&lastSeen=3600").unwrap();
        assert_eq!(q.category, "gym");
        assert_eq!(q.last_seen, 3600);
    }

    #[test]
    fn query_requires_category() {
        // `category` is required (not Option) — a query missing it fails to deser.
        assert!(Query::<ScannerDataQuery>::from_query("lastSeen=10").is_err());
    }

    #[test]
    fn valid_categories_match_points_from_area_arms() {
        // Guard: the up-front allow-list mirrors the koji-scanner query arms.
        assert!(VALID_CATEGORIES.contains(&"gym"));
        assert!(VALID_CATEGORIES.contains(&"pokestop"));
        assert!(VALID_CATEGORIES.contains(&"spawnpoint"));
        assert!(VALID_CATEGORIES.contains(&"station"));
        assert!(VALID_CATEGORIES.contains(&"fort"));
        assert!(!VALID_CATEGORIES.contains(&"bogus"));
    }
}
