//! Integration tests for `lib.rs` route wiring (GROUP 3).
//!
//! Uses `koji_service::test_db_free_app()` which builds the prod App factory
//! with only the DB-free routes mounted. Covers that the routing table wires
//! correctly — not the handler logic itself (those are tested in `v2_no_db.rs`).
//!
//! Routes covered:
//!   - `GET /healthz`               → 200
//!   - `GET /api/v2/openapi.yaml`   → 200 JSON body (valid OpenAPI doc)
//!   - `GET /api/v2/config`         → 200 `{ "status": "ok", "data": { … } }`
//!   - `POST /api/v2/geometry/area` → 200 (routing smoke-test)
//!   - `POST /api/v2/s2/circle-coverage` → 200 (routing smoke-test)
//!   - `GET /api/v2/NOT_A_ROUTE`   → 404 (unknown routes return 404, not panic)

use actix_web::test;

macro_rules! app {
    () => {{ test::init_service(koji_service::test_db_free_app()).await }};
}

async fn body_json(resp: actix_web::dev::ServiceResponse) -> serde_json::Value {
    let bytes = test::read_body(resp).await;
    serde_json::from_slice(&bytes).expect("response must be valid JSON")
}

// ── liveness probe ───────────────────────────────────────────────────────────

#[actix_web::test]
async fn healthz_returns_200() {
    let app = app!();
    let req = test::TestRequest::get().uri("/healthz").to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200, "/healthz must return 200");
}

// ── OpenAPI doc ──────────────────────────────────────────────────────────────

#[actix_web::test]
async fn openapi_yaml_returns_200_with_json_body() {
    let app = app!();
    let req = test::TestRequest::get()
        .uri("/api/v2/openapi.yaml")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    // Body is a valid JSON object (OpenAPI doc emitted as pretty JSON despite the .yaml URL)
    let body = test::read_body(resp).await;
    let doc: serde_json::Value =
        serde_json::from_slice(&body).expect("openapi body must be valid JSON");
    assert!(doc.is_object(), "OpenAPI doc must be a JSON object");
    assert!(
        doc.get("openapi").is_some(),
        "OpenAPI doc must have an 'openapi' key"
    );
    assert!(
        doc.get("paths").is_some(),
        "OpenAPI doc must have a 'paths' key"
    );
}

// ── config endpoint ──────────────────────────────────────────────────────────

#[actix_web::test]
async fn config_route_wired_and_returns_ok_envelope() {
    let app = app!();
    let req = test::TestRequest::get().uri("/api/v2/config").to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200, "/api/v2/config must return 200");
    let v = body_json(resp).await;
    assert_eq!(
        v["status"], "ok",
        "config response must be wrapped in ok envelope"
    );
    assert!(v["data"].is_object(), "config data must be an object");
}

// ── geometry routing ─────────────────────────────────────────────────────────

#[actix_web::test]
async fn geometry_area_route_is_reachable() {
    let app = app!();
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
    assert_eq!(resp.status(), 200, "/api/v2/geometry/area must return 200");
    let v = body_json(resp).await;
    assert_eq!(v["status"], "ok");
    let area = v["data"]["area"].as_f64().expect("area must be numeric");
    assert!(area > 0.0, "area must be positive for a polygon");
}

// ── s2 routing ───────────────────────────────────────────────────────────────

#[actix_web::test]
async fn s2_circle_coverage_route_is_reachable() {
    let app = app!();
    let body = serde_json::json!({
        "lat": 0.0,
        "lon": 0.0,
        "radius": 70.0,
        "level": 15
    });
    let req = test::TestRequest::post()
        .uri("/api/v2/s2/circle-coverage")
        .set_json(&body)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        200,
        "/api/v2/s2/circle-coverage must return 200"
    );
    let v = body_json(resp).await;
    assert_eq!(v["status"], "ok");
}

// ── unknown routes ────────────────────────────────────────────────────────────

#[actix_web::test]
async fn unknown_route_returns_404() {
    let app = app!();
    let req = test::TestRequest::get()
        .uri("/api/v2/this-route-does-not-exist")
        .to_request();
    let resp = test::call_service(&app, req).await;
    // actix returns 404 for unmatched routes
    assert_eq!(resp.status(), 404, "unknown route must return 404");
}

#[actix_web::test]
async fn wrong_method_on_healthz_returns_405() {
    let app = app!();
    // `/healthz` is GET-only; POST should return 405
    let req = test::TestRequest::post().uri("/healthz").to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        405,
        "POST to GET-only /healthz must return 405"
    );
}
