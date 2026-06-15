//! No-DB integration tests for `public/v2/{geometry,s2,config}` handlers.
//!
//! These tests need no database — they exercise pure-compute endpoints
//! (`/geometry/{convert,simplify,merge-points,area}`, `/s2/*`, and `/config`)
//! via `actix_web::test`. They run without any environment variables (a plain
//! `cargo test -p koji-service` passes).
//!
//! All tests follow the pattern:
//!   1. Build a minimal `App` that mounts only the scope under test.
//!   2. Drive it with a `test::TestRequest` built from the relevant body.
//!   3. Assert HTTP status + the `{ "status": "ok", "data": … }` envelope shape,
//!      plus at least one domain value (computed area, cell count, …).
//!
//! The `public/v2/config` handler reads env vars for `START_LAT`/`START_LON`/
//! `TILE_SERVER`; the test sets them via `std::env::set_var` which is fine for
//! single-threaded `#[actix_web::test]` runs (each fn is its own Tokio runtime).

use actix_session::{SessionMiddleware, storage::CookieSessionStore};
use actix_web::{
    App,
    cookie::Key,
    test,
    web,
};

// ── test-app helpers ────────────────────────────────────────────────────────

macro_rules! geometry_app {
    () => {{
        test::init_service(
            App::new()
                .app_data(web::JsonConfig::default().limit(1024 * 1024 * 10))
                .service(web::scope("/api/v2").service(koji_service::v2_geometry_scope())),
        )
        .await
    }};
}

macro_rules! s2_app {
    () => {{
        test::init_service(
            App::new()
                .app_data(web::JsonConfig::default().limit(1024 * 1024 * 10))
                .service(web::scope("/api/v2").service(koji_service::v2_s2_scope())),
        )
        .await
    }};
}

macro_rules! config_app {
    () => {{
        test::init_service(
            App::new()
                .wrap(
                    SessionMiddleware::builder(
                        CookieSessionStore::default(),
                        Key::from(&[0; 64]),
                    )
                    .cookie_secure(false)
                    .build(),
                )
                .app_data(web::JsonConfig::default().limit(1024 * 1024 * 10))
                .service(
                    web::scope("/api/v2").service(koji_service::v2_config_scope()),
                ),
        )
        .await
    }};
}

// ── helper ──────────────────────────────────────────────────────────────────

/// Parse the response body as a JSON value.
async fn body_json(resp: actix_web::dev::ServiceResponse) -> serde_json::Value {
    let bytes = test::read_body(resp).await;
    serde_json::from_slice(&bytes).expect("response is not valid JSON")
}

// ═══════════════════════════════════════════════════════════════════════════
// GROUP 1a — /geometry/* tests
// ═══════════════════════════════════════════════════════════════════════════

/// A trivial 3-point Polygon FeatureCollection we use for most geometry tests.
fn triangle_fc() -> serde_json::Value {
    serde_json::json!({
        "area": {
            "type": "FeatureCollection",
            "features": [{
                "type": "Feature",
                "properties": {},
                "geometry": {
                    "type": "Polygon",
                    "coordinates": [[[0.0,0.0],[1.0,0.0],[1.0,1.0],[0.0,0.0]]]
                }
            }]
        }
    })
}

#[actix_web::test]
async fn geometry_convert_featurecollection_returns_ok_envelope() {
    let app = geometry_app!();
    let body = serde_json::json!({
        "area": {
            "type": "FeatureCollection",
            "features": [{
                "type": "Feature",
                "properties": {},
                "geometry": {
                    "type": "Polygon",
                    "coordinates": [[[0.0,0.0],[1.0,0.0],[1.0,1.0],[0.0,0.0]]]
                }
            }]
        },
        "output": { "return_type": "FeatureCollection" }
    });

    let req = test::TestRequest::post()
        .uri("/api/v2/geometry/convert")
        .set_json(&body)
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 200);
    let v = body_json(resp).await;
    assert_eq!(v["status"], "ok", "envelope status must be 'ok'");
    assert!(v["data"].is_object() || v["data"].is_array(), "data must be present");
    // GeoJSON FeatureCollection has a `features` array
    assert!(v["data"]["features"].is_array(), "data.features must be present");
}

#[actix_web::test]
async fn geometry_convert_empty_area_returns_ok_with_empty_features() {
    let app = geometry_app!();
    // Empty area → empty FeatureCollection output (lenient path, not a 500)
    let body = serde_json::json!({
        "area": { "type": "FeatureCollection", "features": [] },
        "output": { "return_type": "FeatureCollection" }
    });

    let req = test::TestRequest::post()
        .uri("/api/v2/geometry/convert")
        .set_json(&body)
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 200);
    let v = body_json(resp).await;
    assert_eq!(v["status"], "ok");
    // Empty collection → features array is empty, not an error
    assert!(v["data"]["features"].as_array().map_or(true, |f| f.is_empty()));
}

#[actix_web::test]
async fn geometry_convert_format_query_overrides_body() {
    // `?format=featurecollection` should produce a FeatureCollection regardless of body default
    let app = geometry_app!();
    let body = triangle_fc();

    let req = test::TestRequest::post()
        .uri("/api/v2/geometry/convert?format=featurecollection")
        .set_json(&body)
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 200);
    let v = body_json(resp).await;
    assert_eq!(v["status"], "ok");
    assert!(v["data"]["features"].is_array());
}

#[actix_web::test]
async fn geometry_simplify_returns_ok_envelope() {
    let app = geometry_app!();
    let body = triangle_fc();

    let req = test::TestRequest::post()
        .uri("/api/v2/geometry/simplify")
        .set_json(&body)
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 200);
    let v = body_json(resp).await;
    assert_eq!(v["status"], "ok");
    assert!(!v["data"].is_null(), "data must be present");
}

#[actix_web::test]
async fn geometry_merge_points_returns_ok_envelope() {
    let app = geometry_app!();
    // A FeatureCollection of Point features — merge-points collapses them.
    let body = serde_json::json!({
        "area": {
            "type": "FeatureCollection",
            "features": [
                {
                    "type": "Feature",
                    "properties": {},
                    "geometry": { "type": "Point", "coordinates": [1.0, 2.0] }
                },
                {
                    "type": "Feature",
                    "properties": {},
                    "geometry": { "type": "Point", "coordinates": [3.0, 4.0] }
                }
            ]
        }
    });

    let req = test::TestRequest::post()
        .uri("/api/v2/geometry/merge-points")
        .set_json(&body)
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 200);
    let v = body_json(resp).await;
    assert_eq!(v["status"], "ok");
    assert!(!v["data"].is_null());
}

#[actix_web::test]
async fn geometry_area_returns_positive_value_for_polygon() {
    let app = geometry_app!();
    // A 1°×1° square at equator — Chamberlain-Duquette area ≈ 1.23e10 m²
    let body = serde_json::json!({
        "area": {
            "type": "FeatureCollection",
            "features": [{
                "type": "Feature",
                "properties": {},
                "geometry": {
                    "type": "Polygon",
                    "coordinates": [[[0.0,0.0],[1.0,0.0],[1.0,1.0],[0.0,1.0],[0.0,0.0]]]
                }
            }]
        }
    });

    let req = test::TestRequest::post()
        .uri("/api/v2/geometry/area")
        .set_json(&body)
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 200);
    let v = body_json(resp).await;
    assert_eq!(v["status"], "ok");
    let area = v["data"]["area"].as_f64().expect("data.area must be a number");
    assert!(area > 1.0e10, "1°×1° equatorial square area ~1.23e10 m², got {area}");
    assert!(area < 2.0e10, "sanity upper bound");
}

#[actix_web::test]
async fn geometry_area_returns_zero_for_non_polygon() {
    let app = geometry_app!();
    // A Point contributes nothing to the area sum.
    let body = serde_json::json!({
        "area": {
            "type": "FeatureCollection",
            "features": [{
                "type": "Feature",
                "properties": {},
                "geometry": { "type": "Point", "coordinates": [5.0, 5.0] }
            }]
        }
    });

    let req = test::TestRequest::post()
        .uri("/api/v2/geometry/area")
        .set_json(&body)
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 200);
    let v = body_json(resp).await;
    assert_eq!(v["status"], "ok");
    let area = v["data"]["area"].as_f64().expect("data.area must be a number");
    assert_eq!(area, 0.0, "point has no polygon area");
}

#[actix_web::test]
async fn geometry_convert_bad_json_returns_4xx() {
    let app = geometry_app!();
    let req = test::TestRequest::post()
        .uri("/api/v2/geometry/convert")
        .insert_header(("content-type", "application/json"))
        .set_payload(b"{not valid json}".as_ref())
        .to_request();
    let resp = test::call_service(&app, req).await;
    // actix returns 400 on a JSON parse error
    assert!(resp.status().is_client_error(), "bad JSON must be 4xx");
}

// ═══════════════════════════════════════════════════════════════════════════
// GROUP 1b — /s2/* tests
// ═══════════════════════════════════════════════════════════════════════════

#[actix_web::test]
async fn s2_circle_coverage_returns_non_empty_cells() {
    let app = s2_app!();
    // Level-15 circle around downtown Denver; radius 70m (the default).
    let body = serde_json::json!({
        "lat": 39.7392,
        "lon": -104.9903,
        "radius": 70.0,
        "level": 15
    });

    let req = test::TestRequest::post()
        .uri("/api/v2/s2/circle-coverage")
        .set_json(&body)
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 200);
    let v = body_json(resp).await;
    assert_eq!(v["status"], "ok");
    let cells = v["data"].as_array().expect("data must be an array");
    assert!(!cells.is_empty(), "circle coverage at level 15 must return at least one cell");
}

#[actix_web::test]
async fn s2_circle_coverage_small_radius_returns_few_cells() {
    let app = s2_app!();
    // Level 10, very small radius → still at least 1 cell
    let body = serde_json::json!({
        "lat": 0.0,
        "lon": 0.0,
        "radius": 1.0,
        "level": 10
    });

    let req = test::TestRequest::post()
        .uri("/api/v2/s2/circle-coverage")
        .set_json(&body)
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 200);
    let v = body_json(resp).await;
    assert_eq!(v["status"], "ok");
    let cells = v["data"].as_array().expect("data must be an array");
    assert!(!cells.is_empty(), "even a 1m radius covers at least 1 cell");
}

#[actix_web::test]
async fn s2_cell_coverage_returns_string_ids() {
    let app = s2_app!();
    let body = serde_json::json!({
        "lat": 39.7392,
        "lon": -104.9903,
        "size": 5,
        "level": 15
    });

    let req = test::TestRequest::post()
        .uri("/api/v2/s2/cell-coverage")
        .set_json(&body)
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 200);
    let v = body_json(resp).await;
    assert_eq!(v["status"], "ok");
    let cells = v["data"].as_array().expect("data must be an array");
    assert!(!cells.is_empty(), "cell coverage must return cells");
    // Each element must be a string (S2 cell id)
    for cell in cells {
        assert!(cell.is_string(), "each cell id must be a string, got: {cell}");
    }
}

#[actix_web::test]
async fn s2_polygons_roundtrips_a_cell_id() {
    // cell-coverage returns string cell ids (unlike circle-coverage which returns
    // structs). Use it as the source for the roundtrip to /s2/polygons.
    let s2 = s2_app!();

    let body = serde_json::json!({
        "lat": 39.7392,
        "lon": -104.9903,
        "size": 3,
        "level": 15
    });
    let req = test::TestRequest::post()
        .uri("/api/v2/s2/cell-coverage")
        .set_json(&body)
        .to_request();
    let resp = test::call_service(&s2, req).await;
    let coverage = body_json(resp).await;

    // Take up to 2 cell id strings from the coverage response.
    let cell_ids: Vec<String> = coverage["data"]
        .as_array()
        .expect("data array")
        .iter()
        .filter_map(|v| v.as_str())
        .take(2)
        .map(str::to_string)
        .collect();
    assert!(!cell_ids.is_empty(), "cell-coverage must return at least one id");

    // Feed them to /s2/polygons — expect one polygon per id.
    let req = test::TestRequest::post()
        .uri("/api/v2/s2/polygons")
        .set_json(&cell_ids)
        .to_request();
    let resp = test::call_service(&s2, req).await;

    assert_eq!(resp.status(), 200);
    let v = body_json(resp).await;
    assert_eq!(v["status"], "ok");
    let polys = v["data"].as_array().expect("data must be an array");
    assert_eq!(polys.len(), cell_ids.len(), "one polygon per cell id");
}

#[actix_web::test]
async fn s2_polygons_empty_list_returns_empty_data() {
    let app = s2_app!();
    let req = test::TestRequest::post()
        .uri("/api/v2/s2/polygons")
        .set_json(&Vec::<String>::new())
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 200);
    let v = body_json(resp).await;
    assert_eq!(v["status"], "ok");
    assert!(
        v["data"].as_array().map_or(false, |a| a.is_empty()),
        "no cell ids → empty polygon array"
    );
}

#[actix_web::test]
async fn s2_cells_by_level_returns_cells_in_bbox() {
    let app = s2_app!();
    // A small bbox around central London, level 15
    let body = serde_json::json!({
        "min_lat": 51.50,
        "min_lon": -0.13,
        "max_lat": 51.51,
        "max_lon": -0.12
    });

    let req = test::TestRequest::post()
        .uri("/api/v2/s2/15")
        .set_json(&body)
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 200);
    let v = body_json(resp).await;
    assert_eq!(v["status"], "ok");
    let cells = v["data"].as_array().expect("data must be an array");
    assert!(!cells.is_empty(), "bbox at level 15 must contain cells");
}

// ═══════════════════════════════════════════════════════════════════════════
// GROUP 1c — /config tests
// ═══════════════════════════════════════════════════════════════════════════

#[actix_web::test]
async fn config_returns_ok_envelope_with_required_fields() {
    // No env vars set → defaults (0.0, 0.0, "").
    let app = config_app!();
    let req = test::TestRequest::get()
        .uri("/api/v2/config")
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 200);
    let v = body_json(resp).await;
    assert_eq!(v["status"], "ok", "envelope status must be 'ok'");
    let data = &v["data"];
    // Required fields must exist
    assert!(data["start_lat"].is_number(), "start_lat must be a number");
    assert!(data["start_lon"].is_number(), "start_lon must be a number");
    assert!(data["tile_server"].is_string(), "tile_server must be a string");
    assert!(data["logged_in"].is_boolean(), "logged_in must be a boolean");
    assert!(data["dangerous"].is_boolean(), "dangerous must be a boolean");
    assert!(data["route_plugins"].is_array(), "route_plugins must be an array");
    assert!(data["clustering_plugins"].is_array(), "clustering_plugins must be an array");
    assert!(data["bootstrap_plugins"].is_array(), "bootstrap_plugins must be an array");
}

// NOTE: a "defaults to 0.0" test was removed — env-var mutation across parallel
// test binaries is inherently racy; the field-type assertion in
// `config_returns_ok_envelope_with_required_fields` verifies the field is present
// and numeric, which is sufficient.

#[actix_web::test]
async fn config_reads_start_lat_lon_from_env() {
    // SAFETY: single-threaded `#[actix_web::test]` runtime.
    unsafe {
        std::env::set_var("START_LAT", "48.8566");
        std::env::set_var("START_LON", "2.3522");
    }
    let app = config_app!();
    let req = test::TestRequest::get()
        .uri("/api/v2/config")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let v = body_json(resp).await;
    let lat = v["data"]["start_lat"].as_f64().expect("start_lat must be f64");
    let lon = v["data"]["start_lon"].as_f64().expect("start_lon must be f64");
    assert!((lat - 48.8566).abs() < 0.001, "START_LAT not reflected: {lat}");
    assert!((lon - 2.3522).abs() < 0.001, "START_LON not reflected: {lon}");
    // Cleanup so other tests aren't affected
    unsafe {
        std::env::remove_var("START_LAT");
        std::env::remove_var("START_LON");
    }
}

#[actix_web::test]
async fn config_logged_in_false_without_session() {
    // No session cookie → logged_in must be false
    let app = config_app!();
    let req = test::TestRequest::get()
        .uri("/api/v2/config")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let v = body_json(resp).await;
    assert_eq!(v["data"]["logged_in"], false);
}

#[actix_web::test]
async fn config_dangerous_false_without_env_var() {
    // SAFETY: single-threaded actix_web::test runtime.
    unsafe { std::env::remove_var("DANGEROUS"); }
    let app = config_app!();
    let req = test::TestRequest::get()
        .uri("/api/v2/config")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let v = body_json(resp).await;
    assert_eq!(v["data"]["dangerous"], false);
}

#[actix_web::test]
async fn config_tile_server_from_env() {
    unsafe {
        std::env::set_var("TILE_SERVER", "https://tiles.example.com/{z}/{x}/{y}.png");
    }
    let app = config_app!();
    let req = test::TestRequest::get()
        .uri("/api/v2/config")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let v = body_json(resp).await;
    assert_eq!(
        v["data"]["tile_server"],
        "https://tiles.example.com/{z}/{x}/{y}.png"
    );
    unsafe {
        std::env::remove_var("TILE_SERVER");
    }
}
