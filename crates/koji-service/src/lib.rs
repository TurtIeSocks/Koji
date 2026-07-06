use std::{env, io, sync::Arc};

use actix_files::{Files, NamedFile};
use actix_session::{SessionMiddleware, storage::CookieSessionStore};
use actix_web::{
    App, HttpResponse, HttpServer,
    dev::{ServiceRequest, ServiceResponse},
    middleware, web,
};
use actix_web_httpauth::middleware::HttpAuthentication;
use geojson::{Feature, FeatureCollection};

use koji_dragonite::DragoniteClient;
use koji_events::{EventDispatcher, Subscriber, WebhookSubscriber};
use koji_jobs::{HandlerRegistry, JobQueue};
use migration::{DbErr, Migrator, MigratorTrait};
// Re-exported (not a bare `use`) so the `koji-cli` bin can build the same calc
// handler + payload over the job queue. Re-exporting `pub` items through the
// private `mod public` is allowed; `start()` below still references
// `CalculateHandler` via this path.
pub use public::v2::calc::{CALC_KIND, CalcPayload, CalculateHandler};
use utils::{auth, is_docker};

// ── Test-surface re-exports ───────────────────────────────────────────────
//
// Thin wrappers that let integration tests (in `tests/`) build minimal Apps
// from individual scopes without going through the full `start()` factory
// (which requires a live DB, ports, etc.).  Hidden from docs; not feature-
// gated because `#[cfg(test)]` is NOT set for integration-test binaries.

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
    utils::auth::public_validator(req, credentials).await
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
        .wrap(
            actix_session::SessionMiddleware::builder(
                actix_session::storage::CookieSessionStore::default(),
                actix_web::cookie::Key::from(&[0; 64]),
            )
            .cookie_secure(false)
            .build(),
        )
        .service(
            web::scope("/api/v2")
                .service(public::v2::jobs::create_job)
                .service(public::v2::jobs::list_jobs)
                .service(public::v2::jobs::get_job)
                .service(public::v2::jobs::cancel_job)
                .service(public::v2::jobs::algorithms)
                .service(public::v2::geofences::scope())
                .service(public::v2::routes::scope())
                .service(public::v2::plugins::scope())
                .service(public::v2::resources::project::scope())
                .service(public::v2::resources::property::scope())
                .service(public::v2::resources::tile_server::scope())
                .service(public::v2::resources::webhook::scope()),
        )
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
        .wrap(
            actix_session::SessionMiddleware::builder(
                actix_session::storage::CookieSessionStore::default(),
                actix_web::cookie::Key::from(&[0; 64]),
            )
            .cookie_secure(false)
            .build(),
        )
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
    App::new()
        .app_data(web::Data::new(db))
        .service(web::scope("/internal").service(
            web::resource("/geofences")
                .route(web::get().to(internal::geofences::list_rows)),
        ))
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
    App::new()
        .app_data(web::Data::new(db))
        .service(web::scope("/internal").service(
            web::resource("/routes")
                .route(web::get().to(internal::routes::list_rows)),
        ))
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
    let events = Arc::new(EventDispatcher::new(
        db.koji.clone(),
        vec![],
        "test-worker",
    ));
    let hub_arc = Arc::new(hub);
    App::new()
        .app_data(web::Data::new(db))
        .app_data(web::Data::from(hub_arc))
        .app_data(web::Data::from(events))
        .app_data(web::JsonConfig::default().limit(1024 * 1024 * 10))
        .wrap(
            actix_session::SessionMiddleware::builder(
                actix_session::storage::CookieSessionStore::default(),
                actix_web::cookie::Key::from(&[0u8; 64]),
            )
            .cookie_secure(false)
            .build(),
        )
        .service(internal::scope())
}

/// Test surface: the WS handler, re-exported so integration tests in `tests/`
/// can mount it with `web::get().to(koji_service::realtime_ws)`.
/// Function items (unlike closures) implement `actix_web::dev::Handler`.
#[doc(hidden)]
pub use internal::realtime::realtime_ws;

/// Test surface: build a `ServerEvent`.
#[doc(hidden)]
pub fn test_server_event(
    t: &str,
    payload: serde_json::Value,
) -> internal::realtime::ServerEvent {
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
        .wrap(
            actix_session::SessionMiddleware::builder(
                actix_session::storage::CookieSessionStore::default(),
                actix_web::cookie::Key::from(&[0u8; 64]),
            )
            .cookie_secure(false)
            .build(),
        )
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
    let events = Arc::new(EventDispatcher::new(
        db.koji.clone(),
        vec![],
        "test-worker",
    ));
    let hub = Arc::new(internal::realtime::RealtimeHub::new());
    App::new()
        .app_data(web::Data::new(db))
        .app_data(web::Data::from(jobs))
        .app_data(web::Data::from(events))
        .app_data(web::Data::from(hub))
        .app_data(web::JsonConfig::default().limit(1024 * 1024 * 10))
        .wrap(
            actix_session::SessionMiddleware::builder(
                actix_session::storage::CookieSessionStore::default(),
                actix_web::cookie::Key::from(&[0; 64]),
            )
            .cookie_secure(false)
            .build(),
        )
        .service(
            web::scope("/api/v2")
                .service(public::v2::jobs::create_job)
                .service(public::v2::jobs::list_jobs)
                .service(public::v2::jobs::get_job)
                .service(public::v2::jobs::cancel_job)
                .service(public::v2::jobs::algorithms)
                .service(public::v2::geofences::scope())
                .service(public::v2::routes::scope())
                .service(public::v2::plugins::scope()),
        )
        .service(internal::scope())
}

use crate::dragonite::DragoniteSubscriber;

mod dragonite;
mod internal;
mod private;
mod public;
pub mod requests;
mod utils;

/// `GET /api/v2/openapi.yaml` — serve the code-first OpenAPI document.
///
/// Generated from the v2 handlers + DTOs via [`utils::openapi::ApiDoc`] (utoipa),
/// so it can't drift from the implementation. The installed utoipa (5.x, no
/// `yaml` feature) has no `to_yaml`, so the document is emitted as pretty JSON —
/// a valid OpenAPI document body, served under `application/json` (the `.yaml`
/// URL is kept for back-compat with existing doc tooling links). Unauthenticated
/// (like `/healthz`) so tooling can fetch it without the `KOJI_SECRET` bearer.
async fn openapi_spec() -> HttpResponse {
    use utoipa::OpenApi;
    match utils::openapi::ApiDoc::openapi().to_pretty_json() {
        Ok(json) => HttpResponse::Ok()
            .content_type("application/json; charset=utf-8")
            .body(json),
        Err(err) => {
            log::error!("[openapi] failed to render document: {err}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

/// `GET /readyz` — readiness probe. Pings the Koji DB: reachable ⇒ `200`,
/// otherwise `503`. Unauthenticated, like `/healthz` (liveness). `/healthz`
/// answers "is the process up?"; `/readyz` answers "can it serve traffic?".
async fn readyz(db: web::Data<koji_db::KojiDb>) -> HttpResponse {
    match db.koji.ping().await {
        Ok(()) => HttpResponse::Ok().finish(),
        Err(err) => {
            log::warn!("[readyz] DB ping failed: {err}");
            HttpResponse::ServiceUnavailable().finish()
        }
    }
}

/// Build the session cookie key. Production MUST set `KOJI_SESSION_KEY` (≥32 bytes of
/// high-entropy text) so sessions are unforgeable and survive restarts / multiple
/// instances. When unset (or too short) we generate an ephemeral random key and warn —
/// acceptable for local dev only (sessions reset on restart and won't validate across
/// instances).
fn session_key() -> actix_web::cookie::Key {
    use actix_web::cookie::Key;
    match std::env::var("KOJI_SESSION_KEY") {
        Ok(s) if s.len() >= 32 => Key::derive_from(s.as_bytes()),
        Ok(_) => {
            log::warn!(
                "[koji] KOJI_SESSION_KEY is set but < 32 bytes; generating an ephemeral key"
            );
            Key::generate()
        }
        Err(_) => {
            log::warn!(
                "[koji] KOJI_SESSION_KEY unset; generating an ephemeral session key \
                 (dev only — sessions won't survive a restart or load-balance across instances)"
            );
            Key::generate()
        }
    }
}

#[actix_web::main]
pub async fn start() -> io::Result<()> {
    let databases = koji_db::utils::get_database_struct().await;

    match Migrator::up(&databases.koji, None).await {
        Ok(_) => log::info!("Migrations successful"),
        Err(err) => log::error!("Migration Error {:?}", err),
    };

    // Build the process-global plugin registry from disk ∪ the DB `plugin_config`
    // overlay so the runtime dispatch path + `meta/algorithms` reflect saved
    // config from boot. A failure just leaves the lazy disk-only default.
    public::v2::plugins::rebuild_and_install(&databases)
        .await
        .ok();

    // ---- V2 infra: job queue + event dispatcher + optional Dragonite client --
    //
    // The queue runs against the Koji DB (MySQL 8+/MariaDB 10.6+ for SKIP
    // LOCKED). `worker_id` identifies this process in `job.locked_by`.
    let worker_id = format!(
        "{}-{}",
        env::var("HOSTNAME").unwrap_or_else(|_| "koji".to_string()),
        std::process::id()
    );
    // In-process realtime pub/sub hub — built before the job queue so it can be
    // injected as the `JobEventSink`. Shared into every request via app_data.
    // The WS handler at `GET /internal/realtime` reads it to subscribe/publish.
    // Built once here (pre-`HttpServer::new`) and cloned across workers via Arc.
    let hub = Arc::new(internal::realtime::RealtimeHub::new());
    log::info!("[koji] realtime hub initialized");

    // Wire the hub as the job event sink so the worker emits jobs/{id} and jobs
    // realtime events on claim + terminal outcomes + progress ticks.
    let jobs = Arc::new(
        JobQueue::new(databases.koji.clone(), worker_id.clone())
            .with_event_sink(hub.clone() as Arc<dyn koji_jobs::JobEventSink>),
    );

    // Optional Dragonite client, configured purely from env. `None` until both
    // the URL and bearer are set; when present it backs the `DragoniteSubscriber`
    // (and is shared into `AppState` for future direct use).
    let dragonite: Option<DragoniteClient> = match (
        env::var("DRAGONITE_URL").ok(),
        env::var("DRAGONITE_BEARER").ok(),
    ) {
        (Some(url), Some(bearer)) if !url.is_empty() => {
            log::info!("[koji] Dragonite client configured for {url}");
            Some(DragoniteClient::new(url, bearer))
        }
        _ => None,
    };

    // Event dispatcher subscribers (P5): the generic webhook pusher (topic-
    // filtered from `webhook_subscription`) always, plus the Dragonite area
    // pusher when a client is configured. The dispatcher loop is spawned below
    // so `area.*` events emitted by producers are delivered durably.
    let mut subscribers: Vec<Arc<dyn Subscriber>> = vec![Arc::new(
        WebhookSubscriber::with_default_client(databases.koji.clone()),
    )];
    if let Some(client) = &dragonite {
        subscribers.push(Arc::new(DragoniteSubscriber::new(client.clone())));
        log::info!("[koji] Dragonite event subscriber registered");
    }
    let events = Arc::new(EventDispatcher::new(
        databases.koji.clone(),
        subscribers,
        worker_id,
    ));
    // Held for the lifetime of `start()`; the loop claims due outbox rows and
    // delivers to subscribers with backoff/dead-letter.
    let _dispatcher = Arc::clone(&events).spawn();
    log::info!("[koji] event dispatcher spawned");

    // Register the calc handler and spawn the worker pool. `_workers` is held for
    // the lifetime of `start()` so the workers keep running; dropping the set
    // would just detach the tasks (see `WorkerSet`).
    let registry = HandlerRegistry::new().register(CalculateHandler::new(databases.clone()));
    let concurrency = env::var("KOJI_WORKER_CONCURRENCY")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(1);
    let _workers = Arc::clone(&jobs).spawn_workers(concurrency, registry);
    log::info!("[koji] spawned {concurrency} job worker(s)");

    let path = || {
        if is_docker() {
            "./dist"
        } else {
            "../client/dist"
        }
        .to_string()
    };

    HttpServer::new(move || {
        let client = nominatim::Client::new(
            url::Url::parse(
                env::var("NOMINATIM_URL")
                    .unwrap_or("https://nominatim.openstreetmap.org/".to_string())
                    .as_str(),
            )
            .unwrap(),
            "nominatim-rust/0.1.0 test-suite".to_string(),
            None,
        )
        .unwrap();

        App::new()
            .app_data(web::Data::new(databases.clone()))
            .app_data(web::Data::new(client))
            // V2 infra, shared with the worker pool via the same Arc. Existing v1
            // handlers keep their `web::Data<KojiDb>` signatures untouched.
            .app_data(web::Data::from(jobs.clone()))
            .app_data(web::Data::from(events.clone()))
            .app_data(web::Data::new(dragonite.clone()))
            .app_data(web::Data::from(hub.clone()))
            // increase max payload size to 50MB
            .app_data(web::JsonConfig::default().limit(1024 * 1024 * 50))
            .wrap(middleware::Logger::new("%s | %r - %b bytes in %D ms (%a)"))
            .wrap(middleware::Compress::default())
            .wrap(
                SessionMiddleware::builder(CookieSessionStore::default(), session_key())
                    .cookie_secure(std::env::var("KOJI_INSECURE_COOKIES").is_err())
                    .cookie_http_only(true)
                    .cookie_same_site(actix_web::cookie::SameSite::Lax)
                    .build(),
            )
            // OpenAPI 3.1 doc for the v2 surface. Registered before the authed
            // `/api` scope (and unauthenticated) so doc tooling can fetch it
            // without the bearer, and so it matches ahead of the `/v2` scope.
            .service(web::resource("/api/v2/openapi.yaml").route(web::get().to(openapi_spec)))
            // public api
            .service(
                web::scope("/api")
                    // v2: clean, best-practices surface over the job queue. Auth is
                    // applied per-scope.
                    .service(
                        web::scope("/v2")
                            .wrap(HttpAuthentication::with_fn(auth::public_validator))
                            .service(public::v2::jobs::create_job)
                            .service(public::v2::jobs::list_jobs)
                            .service(public::v2::jobs::get_job)
                            .service(public::v2::jobs::cancel_job)
                            .service(public::v2::jobs::algorithms)
                            // Typed CRUD resources. Geometry-bearing geofences/routes
                            // are hand-written (honor `?format=`); projects/
                            // properties/tile-servers are macro-generated plain-ApiResponse
                            // CRUD. Each exposes a `scope()` that wires its own
                            // method+path routing (incl. `/geofences/{id}/publish`).
                            .service(public::v2::geofences::scope())
                            .service(public::v2::routes::scope())
                            .service(public::v2::resources::project::scope())
                            .service(public::v2::resources::property::scope())
                            .service(public::v2::resources::tile_server::scope())
                            .service(public::v2::resources::webhook::scope())
                            // Geometry transforms (convert/simplify/merge-points)
                            // and the S2 cell helpers — split out of the old
                            // `/geo` grab-bag into honest `/geometry` + `/s2`
                            // namespaces. `/geometry/area` (polygon area) rides
                            // the geometry scope.
                            .service(public::v2::geometry::scope())
                            .service(public::v2::s2::scope())
                            // Arbitrary-area golbat-data: POST `/golbat-data/
                            // {category}`(+`/stats`) — the drawn-area markers/count
                            // surface (the saved-fence GET rides geofences::scope()).
                            .service(public::v2::golbat_data::scope())
                            // App bootstrap blob: GET `/config` (map center, tile
                            // server, plugin lists, login state).
                            .service(public::v2::config::config)
                            // Nominatim geocoding proxy: GET `/nominatim?query=`
                            // (Polygon/MultiPolygon results for the import dialog).
                            .service(public::v2::nominatim::search_nominatim)
                            // Plugin management overlay (DB-managed plugin config):
                            // merged disk-manifest + DB-overlay view; PATCH/DELETE
                            // edit only `enabled`/`args_default`/`description`.
                            .service(public::v2::plugins::scope())
                            // Session auth: login/logout/me.
                            .service(public::v2::auth::scope()),
                    ),
            )
            // `/internal` scope: same handlers as `/api/v2` (forward-aliases) +
            // bespoke row-list at `GET /internal/geofences` + WS hub at
            // `GET /internal/realtime`. Behind the same `public_validator` auth.
            // Not in OpenAPI (internal surface only).
            .service(
                internal::scope()
                    .wrap(HttpAuthentication::with_fn(auth::public_validator)),
            )
            // Liveness + readiness probes (top-level, unauthenticated).
            // `/healthz` = process up; `/readyz` = DB reachable (200) or 503.
            .service(web::resource("/healthz").route(web::get().to(HttpResponse::Ok)))
            .service(web::resource("/readyz").route(web::get().to(readyz)))
            .service(
                Files::new("/", path())
                    .index_file("index.html")
                    .default_handler(move |req: ServiceRequest| {
                        // "enables" wildcards for react-router && react-admin
                        let (http_req, _) = req.into_parts();
                        async move {
                            let response = NamedFile::open(format!("{}/index.html", path()))?
                                .into_response(&http_req);
                            Ok(ServiceResponse::new(http_req, response))
                        }
                    }),
            )
    })
    .bind((
        std::env::var("HOST").unwrap_or("0.0.0.0".to_string()),
        std::env::var("PORT")
            .unwrap_or("8080".to_string())
            .parse::<u16>()
            .unwrap(),
    ))?
    .run()
    .await
}
