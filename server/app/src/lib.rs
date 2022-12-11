use std::{env, io};

use actix_files::Files;
use actix_web::{middleware, web, App, HttpServer};
use geojson::{Feature, FeatureCollection};
use sea_orm::{ConnectOptions, Database};

use entity;
use model as models;
mod queries;
mod routes;
mod utils;
use migration::{Migrator, MigratorTrait};

#[actix_web::main]
pub async fn main() -> io::Result<()> {
    dotenv::dotenv().ok();
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

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
            match Database::connect(opt).await {
                Ok(db) => db,
                Err(err) => panic!("{}", err),
            }
        },
        koji_db: {
            let mut opt = ConnectOptions::new(koji_db_url);
            opt.max_connections(max_connections);
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
            match Database::connect(opt).await {
                Ok(db) => Some(db),
                Err(err) => panic!("{}", err),
            }
        },
    };
    Migrator::up(&databases.koji_db, None).await.unwrap();

    let scanner_type = if databases.unown_db.is_none() {
        "rdm"
    } else {
        "unown"
    }
    .to_string();

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(databases.clone()))
            .app_data(web::Data::new(scanner_type.clone()))
            // increase max payload size to 10MB
            .app_data(web::JsonConfig::default().limit(10_485_760))
            .wrap(middleware::Logger::new("%s | %r - %b bytes in %D ms (%a)"))
            .service(
                web::scope("api")
                    .service(routes::misc::config)
                    .service(
                        web::scope("instance")
                            .service(routes::instance::all)
                            .service(routes::instance::instance_type)
                            .service(routes::instance::get_area),
                    )
                    .service(
                        web::scope("data")
                            .service(routes::raw_data::all)
                            .service(routes::raw_data::bound)
                            .service(routes::raw_data::by_area),
                    )
                    .service(
                        web::scope("v1")
                            .service(
                                web::scope("calc")
                                    .service(routes::calculate::bootstrap)
                                    .service(routes::calculate::cluster),
                            )
                            .service(web::scope("convert").service(routes::convert::convert_data))
                            .service(
                                web::scope("geofence")
                                    .service(routes::geofence::all)
                                    .service(routes::geofence::save_koji)
                                    .service(routes::geofence::save_scanner),
                            ),
                    ),
            )
            .service(
                Files::new(
                    "/",
                    if env::var("IS_DOCKER").is_ok() {
                        "./dist"
                    } else {
                        "../client/dist"
                    }
                    .to_string(),
                )
                .index_file("index.html")
                .prefer_utf8(true),
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
