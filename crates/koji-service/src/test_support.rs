//! Test-surface builders: thin wrappers that let integration tests (in
//! `tests/`) build minimal Apps from individual scopes without going through
//! the full `start()` factory (which needs a live DB, ports, etc.). Hidden
//! from docs; not feature-gated because `#[cfg(test)]` is NOT set for
//! integration-test binaries.

use actix_web::{App, HttpResponse, web};
use actix_web_httpauth::middleware::HttpAuthentication;
use koji_events::EventDispatcher;

use crate::utils::auth;
use crate::{internal, openapi_spec, public, v2_db_services};

/// The session middleware every test App shares (fixed key, insecure cookies).
fn test_session_middleware()
-> actix_session::SessionMiddleware<actix_session::storage::CookieSessionStore> {
    actix_session::SessionMiddleware::builder(
        actix_session::storage::CookieSessionStore::default(),
        actix_web::cookie::Key::from(&[0u8; 64]),
    )
    .cookie_secure(false)
    .build()
}

/// The `/geometry` scope (convert / simplify / merge-points / area). No DB.
#[doc(hidden)]
pub fn v2_geometry_scope() -> actix_web::Scope {
    public::v2::geometry::scope()
}

/// The `/s2` scope (circle-coverage / cell-coverage / polygons / {level}). No DB.
#[doc(hidden)]
pub fn v2_s2_scope() -> actix_web::Scope {
    public::v2::s2::scope()
}

/// The `/config` scope (just the single GET handler). Reads env vars; needs the
/// session middleware on the surrounding App (reads `session.get::<bool>("logged_in")`).
/// Returns a `Scope` so it can be mounted with `.service()`.
#[doc(hidden)]
pub fn v2_config_scope() -> actix_web::Scope {
    // `#[get("/config")]` makes `config` an `HttpServiceFactory`, not a plain fn —
    // wrap it in a scope at "/" so tests can mount it under `/api/v2`.
    web::scope("").service(public::v2::config::config)
}

/// Thin wrapper around the `pub(crate)` `utils::auth::public_validator`, exposed
/// `pub` so integration tests in `tests/` can wire it into a test App's scope.
#[doc(hidden)]
pub async fn test_public_validator(
    req: actix_web::dev::ServiceRequest,
    credentials: Option<actix_web_httpauth::extractors::bearer::BearerAuth>,
) -> Result<actix_web::dev::ServiceRequest, (actix_web::Error, actix_web::dev::ServiceRequest)> {
    auth::public_validator(req, credentials).await
}

/// Build a test App that mounts the DB-backed `/api/v2/{geofences,routes,jobs,
/// plugins}` scopes. Caller supplies a live `KojiDb` and a `JobQueue` already
/// connected to the test database. The `EventDispatcher` is constructed with an
/// empty subscriber list so outbox writes still work but nothing is delivered.
///
/// Auth is intentionally bypassed (no `HttpAuthentication` middleware) so the
/// tests don't need a valid bearer token and can focus on CRUD behavior.
#[doc(hidden)]
pub fn test_db_app(
    db: koji_db::KojiDb,
    jobs: std::sync::Arc<koji_jobs::JobQueue>,
) -> actix_web::App<
    impl actix_web::dev::ServiceFactory<
        actix_web::dev::ServiceRequest,
        Config = (),
        Response = actix_web::dev::ServiceResponse,
        Error = actix_web::Error,
        InitError = (),
    >,
> {
    use std::sync::Arc;
    let events = Arc::new(EventDispatcher::new(
        db.koji.clone(),
        vec![], // no-op: no subscribers in tests
        "test-worker",
    ));
    let hub = Arc::new(internal::realtime::RealtimeHub::new());
    App::new()
        .app_data(web::Data::new(db))
        .app_data(web::Data::from(jobs))
        .app_data(web::Data::from(events))
        .app_data(web::Data::from(hub))
        .app_data(web::JsonConfig::default().limit(1024 * 1024 * 10))
        .wrap(test_session_middleware())
        .service(web::scope("/api/v2").configure(v2_db_services))
}

/// Build the DB-free routes of the prod App into an `actix_web::App` for
/// integration testing. Covers `lib.rs` wiring for:
///   - `GET /healthz` (liveness)
///   - `GET /api/v2/openapi.yaml` (doc)
///   - `GET /api/v2/config`, `POST /api/v2/geometry/*`, `POST /api/v2/s2/*`
///
/// Routes that require `KojiDb` (geofences, jobs, etc.) are omitted here —
/// tested separately in the DB-backed group.
#[doc(hidden)]
pub fn test_db_free_app() -> actix_web::App<
    impl actix_web::dev::ServiceFactory<
        actix_web::dev::ServiceRequest,
        Config = (),
        Response = actix_web::dev::ServiceResponse,
        Error = actix_web::Error,
        InitError = (),
    >,
> {
    App::new()
        .app_data(web::JsonConfig::default().limit(1024 * 1024 * 10))
        .wrap(test_session_middleware())
        .service(web::resource("/api/v2/openapi.yaml").route(web::get().to(openapi_spec)))
        .service(
            web::scope("/api/v2")
                .service(public::v2::geometry::scope())
                .service(public::v2::s2::scope())
                .service(web::scope("").service(public::v2::config::config)),
        )
        .service(web::resource("/healthz").route(web::get().to(HttpResponse::Ok)))
}

/// Test surface: a minimal App mounting `GET /internal/geofences` (row list).
#[doc(hidden)]
pub fn test_internal_geofences_app(
    db: koji_db::KojiDb,
) -> actix_web::App<
    impl actix_web::dev::ServiceFactory<
        actix_web::dev::ServiceRequest,
        Config = (),
        Response = actix_web::dev::ServiceResponse,
        Error = actix_web::Error,
        InitError = (),
    >,
> {
    App::new().app_data(web::Data::new(db)).service(
        web::scope("/internal").service(
            web::resource("/geofences").route(web::get().to(internal::geofences::list_rows)),
        ),
    )
}

/// Test surface: a minimal App mounting `GET /internal/routes` (row list).
#[doc(hidden)]
pub fn test_internal_routes_app(
    db: koji_db::KojiDb,
) -> actix_web::App<
    impl actix_web::dev::ServiceFactory<
        actix_web::dev::ServiceRequest,
        Config = (),
        Response = actix_web::dev::ServiceResponse,
        Error = actix_web::Error,
        InitError = (),
    >,
> {
    App::new().app_data(web::Data::new(db)).service(
        web::scope("/internal")
            .service(web::resource("/routes").route(web::get().to(internal::routes::list_rows))),
    )
}

/// Test surface: a minimal App mounting `/api/v2/projects` (the macro-generated
/// `project::scope()`). Used by `tests/internal_geofences_rows.rs` to verify
/// that the `list` handler honors `sortBy/order/q`.
#[doc(hidden)]
pub fn test_projects_app(
    db: koji_db::KojiDb,
) -> actix_web::App<
    impl actix_web::dev::ServiceFactory<
        actix_web::dev::ServiceRequest,
        Config = (),
        Response = actix_web::dev::ServiceResponse,
        Error = actix_web::Error,
        InitError = (),
    >,
> {
    use std::sync::Arc;
    let hub = Arc::new(internal::realtime::RealtimeHub::new());
    App::new()
        .app_data(web::Data::new(db))
        .app_data(web::Data::from(hub))
        .service(web::scope("/api/v2").service(public::v2::resources::project::scope()))
}

/// Test surface: a fresh `RealtimeHub`.
#[doc(hidden)]
pub fn test_realtime_hub() -> internal::realtime::RealtimeHub {
    internal::realtime::RealtimeHub::new()
}

/// Test surface: a full App mounting `/internal` scope (no auth) with `KojiDb`,
/// `RealtimeHub`, and `EventDispatcher` in app_data. Used by
/// `internal_realtime_ws.rs` to verify that mutation handlers publish WS events
/// end-to-end.
///
/// Tests POST to `/internal/geofences` directly — the collection resource now
/// carries both GET and POST so no separate `/api/v2/geofences` scope is needed.
#[doc(hidden)]
pub fn test_internal_live_app(
    db: koji_db::KojiDb,
    hub: internal::realtime::RealtimeHub,
) -> actix_web::App<
    impl actix_web::dev::ServiceFactory<
        actix_web::dev::ServiceRequest,
        Config = (),
        Response = actix_web::dev::ServiceResponse,
        Error = actix_web::Error,
        InitError = (),
    >,
> {
    use std::sync::Arc;
    let events = Arc::new(EventDispatcher::new(db.koji.clone(), vec![], "test-worker"));
    let hub_arc = Arc::new(hub);
    App::new()
        .app_data(web::Data::new(db))
        .app_data(web::Data::from(hub_arc))
        .app_data(web::Data::from(events))
        .app_data(web::JsonConfig::default().limit(1024 * 1024 * 10))
        .wrap(test_session_middleware())
        .service(internal::scope())
}

/// Test surface: the WS handler, re-exported so integration tests in `tests/`
/// can mount it with `web::get().to(koji_service::realtime_ws)`.
/// Function items (unlike closures) implement `actix_web::dev::Handler`.
#[doc(hidden)]
pub use internal::realtime::realtime_ws;

/// Test surface: build a `ServerEvent`.
#[doc(hidden)]
pub fn test_server_event(t: &str, payload: serde_json::Value) -> internal::realtime::ServerEvent {
    internal::realtime::ServerEvent::new(t, payload)
}

/// Test surface: the DB-free members of `/internal` wrapped in real auth middleware.
///
/// Mounts `config`, `auth`, and `realtime` under `/internal` behind
/// `HttpAuthentication::with_fn(public_validator)`. DB-backed routes (geofences,
/// routes, resources, plugins, nominatim) are omitted so the test binary doesn't
/// need a live database. The session middleware is included because `public_validator`
/// reads the `logged_in` session key.
#[doc(hidden)]
pub fn test_internal_authed_app() -> actix_web::App<
    impl actix_web::dev::ServiceFactory<
        actix_web::dev::ServiceRequest,
        Config = (),
        Response = actix_web::dev::ServiceResponse,
        Error = actix_web::Error,
        InitError = (),
    >,
> {
    let hub = std::sync::Arc::new(internal::realtime::RealtimeHub::new());
    App::new()
        .app_data(web::Data::from(hub))
        .app_data(web::JsonConfig::default().limit(1024 * 1024 * 10))
        // Session middleware first (outermost wrap = last applied to request,
        // first to see the response — matches prod ordering).
        .wrap(test_session_middleware())
        // DB-free /internal members only, behind real auth.
        .service(
            web::scope("/internal")
                .wrap(HttpAuthentication::with_fn(auth::public_validator))
                .service(public::v2::config::config)
                .service(public::v2::auth::scope())
                .service(
                    web::resource("/realtime")
                        .route(web::get().to(internal::realtime::realtime_ws)),
                ),
        )
}

/// Like [`test_db_app`] but also mounts `internal::scope()` (no auth, like the
/// public scope) so the parity test can hit both `/api/v2/geofences/{id}` and
/// `/internal/geofences/{id}` in the same `App`.
#[doc(hidden)]
pub fn test_db_app_with_internal(
    db: koji_db::KojiDb,
    jobs: std::sync::Arc<koji_jobs::JobQueue>,
) -> actix_web::App<
    impl actix_web::dev::ServiceFactory<
        actix_web::dev::ServiceRequest,
        Config = (),
        Response = actix_web::dev::ServiceResponse,
        Error = actix_web::Error,
        InitError = (),
    >,
> {
    use std::sync::Arc;
    let events = Arc::new(EventDispatcher::new(db.koji.clone(), vec![], "test-worker"));
    let hub = Arc::new(internal::realtime::RealtimeHub::new());
    App::new()
        .app_data(web::Data::new(db))
        .app_data(web::Data::from(jobs))
        .app_data(web::Data::from(events))
        .app_data(web::Data::from(hub))
        .app_data(web::JsonConfig::default().limit(1024 * 1024 * 10))
        .wrap(test_session_middleware())
        .service(web::scope("/api/v2").configure(v2_db_services))
        .service(internal::scope())
}
