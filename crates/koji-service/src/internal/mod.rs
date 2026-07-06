pub(crate) mod geofences;
pub(crate) mod realtime;
pub(crate) mod routes;

use actix_web::web;

/// The `/internal` scope: bespoke geofence row-list + forward-aliases of the same
/// handlers wired under `/api/v2` (same auth, no OpenAPI doc) + the WS hub.
///
/// Layout:
///   `GET  /internal/geofences`             → `geofences::list_rows` (bespoke)
///   `POST /internal/geofences`             → forwarded `create`
///   `GET  /internal/geofences/{id}`        → forwarded `get_one`
///   `PATCH /internal/geofences/{id}`       → forwarded `update`
///   `DELETE /internal/geofences/{id}`      → forwarded `remove`
///   `POST /internal/geofences/{id}/publish`→ forwarded `publish`
///   `GET  /internal/geofences/{id}/golbat-data` → forwarded `golbat_data`
///   `*    /internal/routes/**`             → forwarded `routes::scope()`
///   `*    /internal/projects/**`           → forwarded `project::scope()`
///   `*    /internal/properties/**`         → forwarded `property::scope()`
///   `*    /internal/tile-servers/**`       → forwarded `tile_server::scope()`
///   `POST /internal/webhooks/{id}/test`    → forwarded `webhooks_test::test_fire`
///   `*    /internal/webhooks/**`           → forwarded `webhook::scope()`
///   `*    /internal/plugins/**`            → forwarded `plugins::scope()`
///   `GET  /internal/config`                → forwarded `config::config`
///   `GET  /internal/nominatim`             → forwarded `nominatim::search_nominatim`
///   `*    /internal/auth/**`               → forwarded `auth::scope()`
///   `GET  /internal/realtime`              → `realtime::realtime_ws` (WS upgrade)
///
/// Mounted behind `HttpAuthentication::with_fn(auth::public_validator)` in
/// `crate::start()` — same auth as `/api/v2`.
pub(crate) fn scope() -> actix_web::Scope {
    use crate::public::v2;
    web::scope("/internal")
        // Bespoke row-list at the collection GET; POST forwards to public create.
        // Both methods on a single resource — actix-web would 405 if GET-only
        // resource and a separate scope both match "/geofences".
        .service(
            web::resource("/geofences")
                .route(web::get().to(geofences::list_rows))
                .route(web::post().to(v2::geofences::create)),
        )
        // All /{id} geofence routes (getOne/patch/delete/publish/golbat-data)
        // forwarded from the public handler, collection POST excluded (handled above).
        .service(v2::geofences::internal_item_scope())
        // Route row-list (bespoke) + POST collection + /{id} CRUD forwarded.
        .service(
            web::resource("/routes")
                .route(web::get().to(routes::list_rows))
                .route(web::post().to(v2::routes::create)),
        )
        .service(v2::routes::internal_item_scope())
        .service(v2::resources::project::scope())
        .service(v2::resources::property::scope())
        .service(v2::resources::tile_server::scope())
        .service(
            web::resource("/webhooks/{id}/test")
                .route(web::post().to(v2::webhooks_test::test_fire)),
        )
        .service(v2::resources::webhook::scope())
        // Plugin overlay CRUD.
        .service(v2::plugins::scope())
        // Single-handler registrations (proc-macro `#[get]` attrs).
        .service(v2::config::config)
        .service(v2::nominatim::search_nominatim)
        // Session auth: login / logout / me.
        .service(v2::auth::scope())
        // WebSocket hub — session-auth gated inside the handler itself.
        .service(
            web::resource("/realtime")
                .route(web::get().to(realtime::realtime_ws)),
        )
        // Atomic bulk import (one tx; dry-run preview).
        .service(
            web::resource("/import")
                .route(web::post().to(v2::import::import_handler)),
        )
        // Geometry normalize/convert (+ simplify/merge-points/area), aliased from
        // /api/v2 so the admin client (which only talks to /internal) can reach it.
        .service(v2::geometry::scope())
}
