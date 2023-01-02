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
use log::LevelFilter;
use sea_orm::{ConnectOptions, Database, DbErr};

use algorithms;
use migration::{Migrator, MigratorTrait};
use model;
use utils::{auth, is_docker};

mod private;
mod public;
mod utils;

#[actix_web::main]
pub async fn main() -> io::Result<()> {
    dotenv::from_filename(env::var("ENV").unwrap_or(".env".to_string())).ok();
    // error | warn | info | debug | trace
    env_logger::init_from_env(
        env_logger::Env::new()
            .default_filter_or(env::var("LOG_LEVEL").unwrap_or("info".to_string())),
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

    let databases = model::KojiDb {
        data_db: {
            let mut opt = ConnectOptions::new(scanner_db_url);
            opt.max_connections(max_connections);
            opt.sqlx_logging_level(LevelFilter::Debug);
            match Database::connect(opt).await {
                Ok(db) => db,
                Err(err) => panic!("Cannot connect to Scanner DB: {}", err),
            }
        },
        koji_db: {
            let mut opt = ConnectOptions::new(koji_db_url);
            opt.max_connections(max_connections);
            opt.sqlx_logging_level(LevelFilter::Debug);
            match Database::connect(opt).await {
                Ok(db) => db,
                Err(err) => panic!("Cannot connect to Koji DB: {}", err),
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
                Err(err) => panic!("Cannot connect to Unown DB: {}", err),
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
        if is_docker().is_ok() {
            "./dist"
        } else {
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
            // private api
            .service(
                web::scope("/config")
                    .service(private::misc::config)
                    .service(private::misc::login)
                    .service(private::misc::logout),
            )
            .service(
                web::scope("/internal")
                    .wrap(HttpAuthentication::with_fn(auth::private_validator))
                    .service(
                        web::scope("/instance")
                            .service(private::data::instance::all)
                            .service(private::data::instance::instance_type)
                            .service(private::data::instance::get_area),
                    )
                    .service(
                        web::scope("/data")
                            .service(private::data::points::all)
                            .service(private::data::points::bound)
                            .service(private::data::points::by_area)
                            .service(private::data::points::area_stats),
                    )
                    .service(
                        web::scope("/admin")
                            .service(private::admin::geofence::get_all)
                            .service(private::admin::geofence::paginate)
                            .service(private::admin::geofence::get_one)
                            .service(private::admin::geofence::create)
                            .service(private::admin::geofence::update)
                            .service(private::admin::geofence::remove)
                            .service(private::admin::project::get_all)
                            .service(private::admin::project::paginate)
                            .service(private::admin::project::get_one)
                            .service(private::admin::project::create)
                            .service(private::admin::project::update)
                            .service(private::admin::project::remove)
                            .service(private::admin::geofence_project::get_all)
                            .service(private::admin::geofence_project::create)
                            .service(private::admin::geofence_project::update)
                            .service(private::admin::geofence_project::update_by_id)
                            .service(private::admin::geofence_project::remove),
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
                                .service(public::v1::geofence::save_koji)
                                .service(public::v1::geofence::save_scanner)
                                .service(public::v1::geofence::specific_return_type)
                                .service(public::v1::geofence::specific_project),
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
