use std::{env, io, sync::Arc};

use actix_session::{SessionMiddleware, storage::CookieSessionStore};
use actix_web::{App, HttpRequest, HttpResponse, HttpServer, middleware, web};
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
use utils::auth;

/// The compiled web app (`apps/web/dist`), copied into `web/` by the Makefile
/// and embedded here. rust-embed's default: a RELEASE build bakes the bytes into
/// the binary (single-file deploy); a DEBUG build reads `web/` live from disk
/// (fast iteration). So embedding is "production only" with no extra gating.
use rust_embed::Embed;

#[derive(Embed)]
#[folder = "web/"]
struct WebAssets;

/// Serve one embedded asset with a guessed content-type and a cache policy
/// (Vite fingerprints `assets/*` → immutable; the HTML shell → no-cache).
fn web_asset(path: &str) -> Option<HttpResponse> {
    let file = WebAssets::get(path)?;
    let mime = mime_guess::from_path(path).first_or_octet_stream();
    let cache = if path.starts_with("assets/") {
        "public, max-age=31536000, immutable"
    } else {
        "no-cache"
    };
    Some(
        HttpResponse::Ok()
            .content_type(mime.to_string())
            .append_header(("Cache-Control", cache))
            .body(file.data.into_owned()),
    )
}

/// Default handler for everything the API scopes don't claim: serve the matching
/// embedded asset, else fall back to `index.html` so client-side routes resolve.
async fn serve_web(req: HttpRequest) -> HttpResponse {
    let raw = req.path().trim_start_matches('/');
    let path = if raw.is_empty() { "index.html" } else { raw };
    web_asset(path)
        .or_else(|| web_asset("index.html"))
        .unwrap_or_else(|| {
            HttpResponse::NotFound().body("web assets not embedded — run `make build`")
        })
}

/// Process bootstrap shared by the server and CLI binaries: load `.env` (or
/// the file named by `ENV`) and initialize `env_logger` from `LOG_LEVEL`
/// (default `info`) to stdout. Was previously copy-pasted per binary, letting
/// ENV/LOG_LEVEL semantics silently drift between them.
pub fn init_env_and_logging() {
    dotenv::from_filename(std::env::var("ENV").unwrap_or_else(|_| ".env".to_string())).ok();
    let mut builder = env_logger::Builder::from_env(
        env_logger::Env::new()
            .default_filter_or(std::env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string())),
    );
    builder.target(env_logger::Target::Stdout);
    builder.init();
}

/// The DB-backed `/api/v2` services (jobs + CRUD resources + webhooks +
/// plugins) shared by prod `start()` and the DB-backed test apps — registered
/// in exactly ONE place so a new resource can't land in prod and silently miss
/// the test apps (or vice versa; the two DB test builders had already drifted).
fn v2_db_services(cfg: &mut web::ServiceConfig) {
    cfg.service(public::v2::jobs::create_job)
        .service(public::v2::jobs::list_jobs)
        .service(public::v2::jobs::get_job)
        .service(public::v2::jobs::cancel_job)
        .service(public::v2::jobs::algorithms)
        // Typed CRUD resources. Geometry-bearing geofences/routes are
        // hand-written (honor `?format=`); projects/properties/tile-servers are
        // macro-generated plain-ApiResponse CRUD. Each exposes a `scope()` that
        // wires its own method+path routing (incl. `/geofences/{id}/publish`).
        .service(public::v2::geofences::scope())
        .service(public::v2::routes::scope())
        .service(public::v2::resources::project::scope())
        .service(public::v2::resources::property::scope())
        .service(public::v2::resources::tile_server::scope())
        .service(
            web::resource("/webhooks/{id}/test")
                .route(web::post().to(public::v2::webhooks_test::test_fire)),
        )
        .service(public::v2::resources::webhook::scope())
        // Plugin management overlay (DB-managed plugin config).
        .service(public::v2::plugins::scope());
}

#[doc(hidden)]
pub mod test_support;
#[doc(hidden)]
pub use test_support::*;

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
        Err(err) => {
            log::error!("Migration Error {:?}", err);
            return Err(io::Error::other(format!(
                "database migration failed, refusing to serve against an un-migrated schema: {err}"
            )));
        }
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

    // Build the nominatim client once, before the server: a malformed
    // NOMINATIM_URL fails startup with a clear error instead of panicking
    // repeatedly inside each worker factory. Client derives Clone.
    let nominatim_url = env::var("NOMINATIM_URL")
        .unwrap_or_else(|_| "https://nominatim.openstreetmap.org/".to_string());
    let nominatim_client = nominatim::Client::new(
        url::Url::parse(&nominatim_url).map_err(|e| {
            io::Error::other(format!("invalid NOMINATIM_URL {nominatim_url:?}: {e}"))
        })?,
        "nominatim-rust/0.1.0 test-suite".to_string(),
        None,
    )
    .map_err(|e| io::Error::other(format!("failed to build nominatim client: {e}")))?;

    let host = std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let port: u16 = port
        .parse()
        .map_err(|e| io::Error::other(format!("invalid PORT {port:?}: {e}")))?;

    HttpServer::new(move || {
        let client = nominatim_client.clone();

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
                            // Jobs + CRUD resources + webhooks + plugins — the
                            // single shared registration list (also mounted by
                            // the DB test apps in `test_support`).
                            .configure(v2_db_services)
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
                            // Session auth: login/logout/me.
                            .service(public::v2::auth::scope()),
                    ),
            )
            // `/internal` scope: same handlers as `/api/v2` (forward-aliases) +
            // bespoke row-list at `GET /internal/geofences` + WS hub at
            // `GET /internal/realtime`. Behind the same `public_validator` auth.
            // Not in OpenAPI (internal surface only).
            .service(internal::scope().wrap(HttpAuthentication::with_fn(auth::public_validator)))
            // Liveness + readiness probes (top-level, unauthenticated).
            // `/healthz` = process up; `/readyz` = DB reachable (200) or 503.
            .service(web::resource("/healthz").route(web::get().to(HttpResponse::Ok)))
            .service(web::resource("/readyz").route(web::get().to(readyz)))
            // Serve the embedded web app for everything the API scopes above
            // don't claim; unknown paths fall back to index.html (SPA routing).
            .default_service(web::route().to(serve_web))
    })
    .bind((host, port))?
    .run()
    .await
}
