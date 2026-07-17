//! Calc-parity wasm exports: the full v2 calc surface (`calc`,
//! `algorithm_options`, `s2_cells`, `convert_geometry`) mirroring the server's
//! `POST /api/v2/jobs` dispatch (`koji-service/src/public/v2/calc.rs::run`),
//! minus the DB/async resolution — the JS caller injects `dataPoints` /
//! `clusters` where the server would have resolved them from the DB (golbat
//! point queries, stored-fence lookups by `instance`/`parent`).
//!
//! Layering mirrors `lib.rs::cluster`'s testability split: the `*_impl` cores
//! take/return `serde_json::Value` (natively testable), and thin
//! `#[wasm_bindgen]` wrappers do the `JsValue` (de)serialization via
//! `serde_wasm_bindgen`. Outbound JSON uses the `json_compatible()` serializer
//! so `serde_json::Value` objects become plain JS objects, not ES Maps.

use algorithms::stats::Stats;
use koji_calc_api::compute::{resolve_cluster_route, run_bootstrap, run_reroute, run_route_stats};
use koji_calc_api::resolve::DEFAULT_RADIUS;
use koji_calc_api::{CalcJobRequest, CalcRequest, GeoInput, area_collection};
use koji_core::{KojiGeometryCollection, Precision};
use serde::Serialize;
use serde_json::json;
use wasm_bindgen::prelude::*;

// ─── inner cores (testable natively) ────────────────────────────────────────

/// The calc core: decode the exact `POST /api/v2/jobs` body and reproduce the
/// server's per-variant dispatch (`calc.rs::run`), returning the server's wire
/// shape `{ "data": <geojson FeatureCollection>, "stats": <Stats> }` (`data` is
/// `null` in benchmark mode, matching the v1 contract).
///
/// Server parity notes:
/// - `area`: `GeoInput` → `FeatureCollection` via the shared lenient
///   [`area_collection`] (exactly what the server enqueue does). The DB-side
///   `create_or_find_collection` (stored fences by `instance`/`parent`) has no
///   wasm equivalent — callers pass the geometry inline.
/// - `dataPoints`/`clusters`: resolved through `enqueue_inputs()`
///   (`resolve_data_points`), never fetched — where the server would query
///   golbat for an empty `dataPoints`, the JS caller injects them instead.
/// - Bootstrap ignores data points (the server enqueues `vec![]` for it), and
///   reroute/route-stats use the pre-supplied sets as-is — both already fall
///   out of `enqueue_inputs()`.
pub(crate) fn calc_impl(value: serde_json::Value) -> Result<serde_json::Value, String> {
    let body: CalcJobRequest =
        serde_json::from_value(value).map_err(|e| format!("invalid calc request: {e}"))?;
    let req = body.request;

    // The same input extraction the server enqueue runs (area still a GeoInput;
    // dataPoints/clusters resolved to flat `[lat, lon]` lists).
    let inputs = req.enqueue_inputs();
    let area = area_collection(&inputs.area);
    let data_points = inputs.data_points;
    let clusters = inputs.clusters;

    // Per-variant dispatch — mirrors `CalculateHandler::run` exactly.
    let (benchmark_mode, collection, stats): (bool, KojiGeometryCollection, Stats) = match req {
        // `Cluster` and `Route` share `ClusterReq`; only `Route` applies the
        // `sort_by Unset -> Tsp` override (spec §2 table).
        CalcRequest::Cluster(c) => resolve_cluster_route(c, false, &data_points, area),
        CalcRequest::Route(c) => resolve_cluster_route(c, true, &data_points, area),
        CalcRequest::Reroute(r) => {
            let benchmark_mode = r.dev.resolve().benchmark_mode;
            let routing_config = r.routing.resolve();
            let radius = r.radius.unwrap_or(DEFAULT_RADIUS);
            let instance = r.instance.unwrap_or_default();
            let (collection, stats) =
                run_reroute(clusters, data_points, radius, &routing_config, &instance);
            (benchmark_mode, collection, stats)
        }
        CalcRequest::Bootstrap(b) => {
            let benchmark_mode = b.dev.resolve().benchmark_mode;
            let bootstrap_config = b.bootstrap.resolve();
            let routing_config = b.routing.resolve();
            let instance = b.instance.unwrap_or_default();
            let (collection, stats) =
                run_bootstrap(area, &bootstrap_config, &routing_config, &instance)?;
            (benchmark_mode, collection, stats)
        }
        CalcRequest::RouteStats(s) => {
            let benchmark_mode = s.dev.resolve().benchmark_mode;
            let radius = s.radius.unwrap_or(DEFAULT_RADIUS);
            let min_points = s.min_points.unwrap_or(1);
            let instance = s.instance.unwrap_or_default();
            let (collection, stats) =
                run_route_stats(clusters, data_points, radius, min_points, &instance);
            (benchmark_mode, collection, stats)
        }
    };

    // Benchmark mode returns only the stats (the v1 contract); otherwise the
    // result carries both the geojson and the stats. The external wire shape
    // stays a geojson `FeatureCollection`: project the `KojiGeometryCollection`
    // at the boundary via the Phase 1 outbound `From` — the server's exact
    // result shaping.
    let data = if benchmark_mode {
        serde_json::Value::Null
    } else {
        let fc = geojson::FeatureCollection::from(&collection);
        serde_json::to_value(&fc).map_err(|e| format!("failed to serialize result: {e}"))?
    };
    Ok(json!({ "data": data, "stats": stats }))
}

/// The available clustering / routing / bootstrap modes — the wasm mirror of
/// `GET /api/v2/algorithms` (plugins excluded on wasm: the non-native
/// `*_plugins()` fallbacks return empty lists).
pub(crate) fn options_impl() -> serde_json::Value {
    json!({
        "clustering": algorithms::clustering::all_clustering_options(),
        "routing": algorithms::routing::all_routing_options(),
        "bootstrap": algorithms::bootstrap::all_bootstrap_options(),
    })
}

/// Normalize an inbound geojson container (`FeatureCollection` / `Feature` /
/// `Geometry` — the calc `area` shapes) through `KojiGeometryCollection` and
/// project back out as a `FeatureCollection`.
pub(crate) fn convert_impl(value: serde_json::Value) -> Result<serde_json::Value, String> {
    let input: GeoInput =
        serde_json::from_value(value).map_err(|e| format!("invalid geometry: {e}"))?;
    let coll = input
        .to_koji()
        .map_err(|e| format!("geometry conversion failed: {e}"))?;
    let fc = geojson::FeatureCollection::from(&coll);
    serde_json::to_value(&fc).map_err(|e| format!("failed to serialize result: {e}"))
}

// ─── wasm boundary ───────────────────────────────────────────────────────────

/// Serialize to a plain-JS-object `JsValue` (`json_compatible()`: object maps
/// become `{}` objects rather than ES2015 `Map`s).
fn to_js<T: Serialize>(value: &T) -> Result<JsValue, JsError> {
    let serializer = serde_wasm_bindgen::Serializer::json_compatible();
    Ok(value.serialize(&serializer)?)
}

/// Run a calc request — the wasm mirror of `POST /api/v2/jobs`. `req` is the
/// exact jobs body (`buildCalcBody` output) with `dataPoints`/`clusters`
/// injected by the caller where the server would resolve them from the DB.
/// Returns `{ data: <geojson FeatureCollection>, stats: <Stats> }`.
#[wasm_bindgen]
pub fn calc(req: JsValue) -> Result<JsValue, JsError> {
    let value: serde_json::Value = serde_wasm_bindgen::from_value(req)?;
    let out = calc_impl(value).map_err(|e| JsError::new(&e))?;
    to_js(&out)
}

/// Available algorithm modes: `{ clustering: string[], routing: string[],
/// bootstrap: string[] }` — the wasm mirror of `GET /api/v2/algorithms`.
#[wasm_bindgen]
pub fn algorithm_options() -> JsValue {
    to_js(&options_impl()).expect("static option lists always serialize")
}

/// S2 cells covering a bbox at `level` — the wasm mirror of
/// `POST /api/v2/s2/{cell_level}`. Returns `Vec<S2Response>`
/// (`{ id, coords: [[lat, lon]; 4] }`).
#[wasm_bindgen]
pub fn s2_cells(
    level: u8,
    min_lat: Precision,
    min_lon: Precision,
    max_lat: Precision,
    max_lon: Precision,
) -> JsValue {
    let cells = koji_core::s2::get_cells(level, min_lat, min_lon, max_lat, max_lon);
    to_js(&cells).expect("S2 responses always serialize")
}

/// Normalize a geojson container (FC / Feature / Geometry) through
/// `KojiGeometryCollection` and return the normalized `FeatureCollection`.
#[wasm_bindgen]
pub fn convert_geometry(area: JsValue) -> Result<JsValue, JsError> {
    let value: serde_json::Value = serde_wasm_bindgen::from_value(area)?;
    let out = convert_impl(value).map_err(|e| JsError::new(&e))?;
    to_js(&out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calc_route_mode_end_to_end() {
        let body = serde_json::json!({
            "mode": "route",
            "category": "spawnpoint",
            "area": { "type": "FeatureCollection", "features": [] },
            "dataPoints": [[40.0, -74.0], [40.0001, -74.0], [40.0002, -74.0]],
            "clustering": { "calculationMode": "radius", "radius": 200, "minPoints": 1 },
            "routing": { "sortBy": "random" }
        });
        let out = calc_impl(body).expect("calc should succeed");
        assert_eq!(out["data"]["type"], "FeatureCollection");
        assert!(out["stats"]["total_clusters"].as_u64().unwrap() >= 1);
    }

    #[test]
    fn calc_bootstrap_mode_end_to_end() {
        // A small square around lower Manhattan; radius bootstrap.
        let body = serde_json::json!({
            "mode": "bootstrap",
            "area": { "type": "FeatureCollection", "features": [{
                "type": "Feature", "properties": {},
                "geometry": { "type": "Polygon", "coordinates": [[
                    [-74.02, 40.70], [-74.00, 40.70], [-74.00, 40.72], [-74.02, 40.72], [-74.02, 40.70]
                ]]}
            }]},
            "bootstrap": { "calculationMode": "radius", "radius": 300 }
        });
        let out = calc_impl(body).expect("bootstrap should succeed");
        assert_eq!(out["data"]["type"], "FeatureCollection");
        assert!(!out["data"]["features"].as_array().unwrap().is_empty());
    }

    #[test]
    fn calc_route_stats_mode() {
        let body = serde_json::json!({
            "mode": "routeStats",
            "dataPoints": [[40.0, -74.0], [40.0001, -74.0]],
            "clusters": [[40.00005, -74.0]],
            "radius": 100, "minPoints": 1
        });
        let out = calc_impl(body).expect("routeStats should succeed");
        assert!(out["stats"]["total_points"].as_u64().unwrap() == 2);
    }

    #[test]
    fn options_match_algorithms_crate() {
        let o = options_impl();
        assert!(
            o["clustering"]
                .as_array()
                .unwrap()
                .iter()
                .any(|v| v == "balanced")
        );
        assert!(o["routing"].as_array().unwrap().iter().any(|v| v == "tsp"));
        assert!(
            o["bootstrap"]
                .as_array()
                .unwrap()
                .iter()
                .any(|v| v == "radius")
        );
    }

    // ── supplementary coverage (beyond the brief's four) ─────────────────────

    /// Benchmark mode nulls out `data` (the v1 contract the server keeps).
    #[test]
    fn calc_benchmark_mode_nulls_data() {
        let body = serde_json::json!({
            "mode": "cluster",
            "dataPoints": [[40.0, -74.0], [40.0001, -74.0]],
            "clustering": { "radius": 200, "minPoints": 1 },
            "dev": { "benchmarkMode": true }
        });
        let out = calc_impl(body).expect("benchmark calc should succeed");
        assert!(out["data"].is_null());
        assert!(out["stats"]["total_clusters"].as_u64().unwrap() >= 1);
    }

    /// Reroute with no `dataPoints`: legacy compat treats `clusters` as the
    /// route to sort (the shared `run_reroute` empty-clusters swap, exercised
    /// in the inverse direction — clusters supplied, points absent).
    #[test]
    fn calc_reroute_mode_routes_existing_clusters() {
        let body = serde_json::json!({
            "mode": "reroute",
            "clusters": [[40.0, -74.0], [40.001, -74.0], [40.0005, -74.0]],
            "routing": { "sortBy": "random" }
        });
        let out = calc_impl(body).expect("reroute should succeed");
        assert_eq!(out["data"]["type"], "FeatureCollection");
        assert_eq!(out["stats"]["total_clusters"].as_u64().unwrap(), 3);
    }

    /// A garbage body surfaces a decode error, not a panic.
    #[test]
    fn calc_invalid_body_is_err() {
        let out = calc_impl(serde_json::json!({ "mode": "no-such-mode" }));
        assert!(out.is_err());
        assert!(out.unwrap_err().contains("invalid calc request"));
    }

    /// convert: a bare Polygon geometry normalizes into a one-feature FC.
    #[test]
    fn convert_geometry_polygon_normalizes_to_fc() {
        let area = serde_json::json!({
            "type": "Polygon",
            "coordinates": [[[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0], [0.0, 0.0]]]
        });
        let out = convert_impl(area).expect("convert should succeed");
        assert_eq!(out["type"], "FeatureCollection");
        assert_eq!(out["features"].as_array().unwrap().len(), 1);
    }

    /// convert: non-geojson input is an error, not a panic.
    #[test]
    fn convert_geometry_invalid_input_is_err() {
        let out = convert_impl(serde_json::json!({ "nope": true }));
        assert!(out.is_err());
    }

    /// s2 core: the level-15 covering of a small bbox is non-empty and every
    /// cell has a numeric id string (native check of the fn the wrapper calls).
    #[test]
    fn s2_get_cells_covers_small_bbox() {
        let cells = koji_core::s2::get_cells(15, 40.70, -74.02, 40.72, -74.00);
        assert!(!cells.is_empty());
        assert!(cells.iter().all(|c| c.id.parse::<u64>().is_ok()));
    }
}
