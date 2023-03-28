use std::{env, fs, io};

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
use migration::{DbErr, Migrator, MigratorTrait};
use model;
use utils::{auth, is_docker};

mod private;
mod public;
mod utils;

#[actix_web::main]
pub async fn start() -> io::Result<()> {
    let databases = model::utils::get_database_struct().await;

    match Migrator::up(&databases.koji_db, None).await {
        Ok(_) => log::info!("Migrations successful"),
        Err(err) => log::error!("Migration Error {:?}", err),
    };

    let scanner_type = if databases.unown_db.is_none() {
        "rdm"
    } else {
        "unown"
    }
    .to_string();

    let path = || {
        if is_docker().is_ok() {
            "./dist"
        } else {
            "../client/dist"
        }
        .to_string()
    };

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

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(databases.clone()))
            .app_data(web::Data::new(scanner_type.clone()))
            .app_data(web::Data::new(client.clone()))
            // increase max payload size to 20MB
            .app_data(web::JsonConfig::default().limit(20_971_520))
            .wrap(middleware::Logger::new("%s | %r - %b bytes in %D ms (%a)"))
            .wrap(middleware::Compress::default())
            .wrap(
                SessionMiddleware::builder(CookieSessionStore::default(), Key::from(&[0; 64]))
                    .cookie_secure(false)
                    .build(),
            )
            // private api
            .service(
                web::scope("/config")
                    .service(private::misc::config)
                    .service(private::misc::login)
                    .service(private::misc::logout)
                    .service(private::misc::search_nominatim),
            )
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
                            .service(private::admin::get_all)
                            .service(private::admin::search)
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
                            web::scope("/calc")
                                .service(public::v1::calculate::bootstrap)
                                .service(public::v1::calculate::reroute)
                                .service(public::v1::calculate::cluster)
                                .service(public::v1::calculate::calculate_area),
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
                                .service(public::v1::geofence::save_koji)
                                .service(public::v1::geofence::save_scanner)
                                .service(public::v1::geofence::push_to_prod)
                                .service(public::v1::geofence::get_area)
                                .service(public::v1::geofence::specific_return_type)
                                .service(public::v1::geofence::specific_project),
                        )
                        .service(
                            web::scope("/route")
                                .service(public::v1::route::all)
                                .service(public::v1::route::save_koji)
                                .service(public::v1::route::push_to_prod)
                                .service(public::v1::route::get_area)
                                .service(public::v1::route::specific_return_type)
                                .service(public::v1::route::specific_project),
                        )
                        .service(
                            web::scope("/s2")
                                .service(public::v1::s2::circle_coverage)
                                .service(public::v1::s2::cell_coverage)
                                .service(public::v1::s2::cell_polygons)
                                .service(public::v1::s2::s2_cells),
                        ),
                ),
            )
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
