use std::{env, io};

use actix_files::{Files, NamedFile};
use actix_session::{storage::CookieSessionStore, SessionMiddleware};
use actix_web::{
    cookie::Key,
    dev::{ServiceRequest, ServiceResponse},
    middleware, web, App, HttpServer,
};
use actix_web_httpauth::middleware::HttpAuthentication;
use geojson::{Feature, FeatureCollection};
use log::LevelFilter;
use sea_orm::{ConnectOptions, Database};

use entity;
use model as models;
mod queries;
mod routes;
mod utils;
use migration::{Migrator, MigratorTrait};
use utils::auth;

#[actix_web::main]
pub async fn main() -> io::Result<()> {
    dotenv::from_filename(env::var("ENV").unwrap_or(".env".to_string())).ok();
    // error | warn | info | debug | trace
    env_logger::init_from_env(
        env_logger::Env::new()
            .default_filter_or(env::var("LOG_LEVEL").unwrap_or("warn".to_string())),
    );

    let koji_db_url = env::var("KOJI_DB_URL").expect("Need KOJI_DB_URL env var to run migrations");
    let scanner_db_url = if env::var("DATABASE_URL").is_ok() {
        println!("[WARNING] `DATABASE_URL` is deprecated in favor of `SCANNER_DB_URL`");
        env::var("DATABASE_URL")
    } else {
        env::var("SCANNER_DB_URL")
    }
    .expect("Need SCANNER_DB_URL env var");

    let max_connections: u32 = if let Ok(parsed) = env::var("MAX_CONNECTIONS")
        .unwrap_or("100".to_string())
        .parse()
    {
        parsed
    } else {
        100
    };
    let unown_db_url = env::var("UNOWN_DB").unwrap_or("".to_string());

    let databases = models::KojiDb {
        data_db: {
            let mut opt = ConnectOptions::new(scanner_db_url);
            opt.max_connections(max_connections);
            opt.sqlx_logging_level(LevelFilter::Debug);
            match Database::connect(opt).await {
                Ok(db) => db,
                Err(err) => panic!("{}", err),
            }
        },
        koji_db: {
            let mut opt = ConnectOptions::new(koji_db_url);
            opt.max_connections(max_connections);
            opt.sqlx_logging_level(LevelFilter::Debug);
            match Database::connect(opt).await {
                Ok(db) => db,
                Err(err) => panic!("{}", err),
            }
        },
        unown_db: if unown_db_url.is_empty() {
            None
        } else {
            let mut opt = ConnectOptions::new(unown_db_url);
            opt.max_connections(max_connections);
            opt.sqlx_logging_level(LevelFilter::Debug);
            match Database::connect(opt).await {
                Ok(db) => Some(db),
                Err(err) => panic!("{}", err),
            }
        },
    };
    match Migrator::up(&databases.koji_db, None).await {
        Ok(_) => println!("Migrations successful"),
        Err(err) => println!("Migration Error {:?}", err),
    };

    let scanner_type = if databases.unown_db.is_none() {
        "rdm"
    } else {
        "unown"
    }
    .to_string();

    let path = || {
        if env::var("HOME").unwrap_or("".to_string()).eq("/root") {
            // docker path
            "./dist"
        } else {
            // repo path
            "../client/dist"
        }
        .to_string()
    };

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(databases.clone()))
            .app_data(web::Data::new(scanner_type.clone()))
            // increase max payload size to 20MB
            .app_data(web::JsonConfig::default().limit(20_971_520))
            .wrap(middleware::Logger::new("%s | %r - %b bytes in %D ms (%a)"))
            .wrap(
                SessionMiddleware::builder(CookieSessionStore::default(), Key::from(&[0; 64]))
                    .cookie_secure(false)
                    .build(),
            )
            .service(routes::misc::config)
            .service(routes::misc::login)
            // private api
            .service(
                web::scope("/internal")
                    .wrap(HttpAuthentication::with_fn(auth::pri_validator))
                    .service(
                        web::scope("/instance")
                            .service(routes::instance::all)
                            .service(routes::instance::instance_type)
                            .service(routes::instance::get_area),
                    )
                    .service(
                        web::scope("/data")
                            .service(routes::raw_data::all)
                            .service(routes::raw_data::bound)
                            .service(routes::raw_data::by_area)
                            .service(routes::raw_data::area_stats),
                    )
                    .service(
                        web::scope("/admin")
                            .service(routes::admin::geofence::get_all)
                            .service(routes::admin::geofence::paginate)
                            .service(routes::admin::geofence::get_one)
                            .service(routes::admin::geofence::create)
                            .service(routes::admin::geofence::update)
                            .service(routes::admin::geofence::remove)
                            .service(routes::admin::project::get_all)
                            .service(routes::admin::project::paginate)
                            .service(routes::admin::project::get_one)
                            .service(routes::admin::project::create)
                            .service(routes::admin::project::update)
                            .service(routes::admin::project::remove)
                            .service(routes::admin::geofence_project::get_all)
                            .service(routes::admin::geofence_project::create)
                            .service(routes::admin::geofence_project::update)
                            .service(routes::admin::geofence_project::update_by_id)
                            .service(routes::admin::geofence_project::remove),
                    ),
            )
            // public api
            .service(
                web::scope("/api").service(
                    web::scope("/v1")
                        .wrap(HttpAuthentication::with_fn(auth::pub_validator))
                        .service(
                            web::scope("/calc")
                                .service(routes::calculate::bootstrap)
                                .service(routes::calculate::cluster),
                        )
                        .service(web::scope("/convert").service(routes::convert::convert_data))
                        .service(
                            web::scope("/geofence")
                                .service(routes::geofence::all)
                                .service(routes::geofence::save_koji)
                                .service(routes::geofence::save_scanner)
                                .service(routes::geofence::specific_return_type)
                                .service(routes::geofence::specific_project),
                        ),
                ),
            )
            .service(
                Files::new("/", path())
                    .index_file("index.html")
                    .default_handler(move |req: ServiceRequest| {
                        // "enables" wildcards for react-router/react-admin
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
