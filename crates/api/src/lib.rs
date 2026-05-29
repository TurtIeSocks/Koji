use std::{env, fs, io, sync::Arc};

use actix_files::{Files, NamedFile};
use actix_session::{storage::CookieSessionStore, SessionMiddleware};
use actix_web::{
    cookie::Key,
    delete,
    dev::{ServiceRequest, ServiceResponse},
    get, middleware, patch, post, web, App, Error, HttpResponse, HttpServer,
};
use actix_web_httpauth::middleware::HttpAuthentication;
use geojson::{Feature, FeatureCollection};
use log;
use nominatim;

use algorithms;
use koji_dragonite::DragoniteClient;
use koji_events::{EventDispatcher, Subscriber, WebhookSubscriber};
use koji_jobs::{HandlerRegistry, JobQueue};
use migration::{DbErr, Migrator, MigratorTrait};
use model;
use public::v2::calc::CalculateHandler;
use utils::{auth, is_docker};

use crate::dragonite::DragoniteSubscriber;

mod dragonite;
mod private;
mod public;
mod utils;

#[actix_web::main]
pub async fn start() -> io::Result<()> {
    let databases = koji_db::utils::get_database_struct().await;

    match Migrator::up(&databases.koji, None).await {
        Ok(_) => log::info!("Migrations successful"),
        Err(err) => log::error!("Migration Error {:?}", err),
    };

    // ---- V2 infra: job queue + event dispatcher + optional Dragonite client --
    //
    // The queue runs against the Koji DB (MySQL 8+/MariaDB 10.6+ for SKIP
    // LOCKED). `worker_id` identifies this process in `job.locked_by`.
    let worker_id = format!(
        "{}-{}",
        env::var("HOSTNAME").unwrap_or_else(|_| "koji".to_string()),
        std::process::id()
    );
    let jobs = Arc::new(JobQueue::new(databases.koji.clone(), worker_id.clone()));

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
        if is_docker().is_ok() {
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
            // increase max payload size to 50MB
            .app_data(web::JsonConfig::default().limit(1024 * 1024 * 50))
            .wrap(middleware::Logger::new("%s | %r - %b bytes in %D ms (%a)"))
            .wrap(middleware::Compress::default())
            .wrap(
                SessionMiddleware::builder(CookieSessionStore::default(), Key::from(&[0; 64]))
                    .cookie_secure(false)
                    .build(),
            )
            .service(web::resource("/health").route(web::get().to(|| HttpResponse::Ok())))
            .service(
                web::scope("/config")
                    .service(private::misc::config)
                    .service(private::misc::login)
                    .service(private::misc::logout)
                    .service(private::misc::search_nominatim),
            )
            // private api
            .service(
                web::scope("/internal")
                    .wrap(HttpAuthentication::with_fn(auth::private_validator))
                    .service(
                        web::scope("/routes")
                            .service(private::instance::from_koji)
                            .service(private::instance::from_scanner)
                            .service(private::instance::route_from_db),
                    )
                    .service(
                        web::scope("/data")
                            .service(private::points::all)
                            .service(private::points::bound)
                            .service(private::points::by_area)
                            .service(private::points::area_stats),
                    )
                    .service(
                        web::scope("/admin")
                            .service(private::admin::paginate)
                            .service(private::admin::parent_list)
                            .service(private::admin::get_all)
                            .service(private::admin::search)
                            .service(private::admin::assign)
                            .service(private::admin::get_one)
                            .service(private::admin::create)
                            .service(private::admin::update)
                            .service(private::admin::remove)
                            .service(
                                // TODO: Consolidate with the above endpoints
                                web::scope("/geofence_project")
                                    .service(private::geofence_project::get_all)
                                    .service(private::geofence_project::create)
                                    .service(private::geofence_project::update)
                                    .service(private::geofence_project::update_by_id)
                                    .service(private::geofence_project::remove),
                            ),
                    ),
            )
            // public api
            .service(
                web::scope("/api").service(
                    web::scope("/v1")
                        .wrap(HttpAuthentication::with_fn(auth::public_validator))
                        .service(
                            web::resource("/health").route(web::get().to(|| HttpResponse::Ok())),
                        )
                        .service(
                            web::scope("/calc")
                                .service(public::v1::calculate::bootstrap)
                                .service(public::v1::calculate::route_stats)
                                .service(public::v1::calculate::route_stats_category)
                                .service(public::v1::calculate::reroute)
                                .service(public::v1::calculate::calculate_area)
                                .service(public::v1::calculate::cluster),
                        )
                        .service(
                            web::scope("/convert")
                                .service(public::v1::convert::convert_data)
                                .service(public::v1::convert::merge_points)
                                .service(public::v1::convert::simplify),
                        )
                        .service(
                            web::scope("/geofence")
                                .service(public::v1::geofence::all)
                                .service(public::v1::geofence::reference_data)
                                .service(public::v1::geofence::reference_data_project)
                                .service(public::v1::geofence::save_koji)
                                .service(public::v1::geofence::save_scanner)
                                .service(public::v1::geofence::remove)
                                .service(public::v1::geofence::push_to_prod)
                                .service(public::v1::geofence::get_area)
                                .service(public::v1::geofence::specific_return_type)
                                .service(public::v1::geofence::specific_project),
                        )
                        .service(
                            web::scope("/route")
                                .service(public::v1::route::all)
                                .service(public::v1::route::reference_data)
                                .service(public::v1::route::reference_data_geofence)
                                .service(public::v1::route::save_koji)
                                .service(public::v1::route::push_to_prod)
                                .service(public::v1::route::get_area)
                                .service(public::v1::route::specific_return_type)
                                .service(public::v1::route::specific_geofence),
                        )
                        .service(web::scope("/project").service(public::v1::project::push_to_prod))
                        .service(
                            web::scope("/s2")
                                .service(public::v1::s2::circle_coverage)
                                .service(public::v1::s2::cell_coverage)
                                .service(public::v1::s2::cell_polygons)
                                .service(public::v1::s2::s2_cells),
                        )
                        .service(web::scope("/info").service(public::v1::info::main)),
                )
                // v2: clean, best-practices surface over the job queue. Auth is
                // applied per-scope (mirrors v1's public_validator).
                .service(
                    web::scope("/v2")
                        .wrap(HttpAuthentication::with_fn(auth::public_validator))
                        .service(public::v2::jobs::enqueue_job)
                        .service(public::v2::jobs::get_job)
                        .service(public::v2::jobs::cancel_job)
                        .service(public::v2::jobs::calc_mode_category)
                        .service(public::v2::jobs::calc_mode)
                        .service(public::v2::jobs::meta_algorithms)
                        // Typed CRUD resources. Geometry-bearing geofences/routes
                        // are hand-written (honor `?format=`); projects/
                        // properties/tile-servers are macro-generated plain-JSend
                        // CRUD. Each exposes a `scope()` that wires its own
                        // method+path routing (incl. `/geofences/{id}/publish`).
                        .service(public::v2::geofences::scope())
                        .service(public::v2::routes::scope())
                        .service(public::v2::resources::project::scope())
                        .service(public::v2::resources::property::scope())
                        .service(public::v2::resources::tile_server::scope()),
                ),
            )
            // Liveness probe (top-level, unauthenticated — mirrors `/health`).
            .service(web::resource("/healthz").route(web::get().to(|| HttpResponse::Ok())))
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
