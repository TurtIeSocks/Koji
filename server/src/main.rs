#[macro_use]
extern crate diesel;

use actix_files::Files;
use actix_web::{middleware, web, App, HttpServer};
use diesel::prelude::*;
use diesel::r2d2::{self, ConnectionManager};

pub type DbPool = r2d2::Pool<ConnectionManager<MysqlConnection>>;

mod cpp;
mod db;
mod models;
mod queries;
mod routes;
mod utils;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL");
    let scanner_type = std::env::var("SCANNER_TYPE").unwrap_or("rdm".to_string());

    let manager = ConnectionManager::<MysqlConnection>::new(database_url);
    let pool: DbPool = r2d2::Pool::builder()
        .build(manager)
        .expect("Failed to create pool.");
    let port = std::env::var("PORT").unwrap_or("8080".to_string());
    let serve_from = if std::env::var("NODE_ENV") == Ok("development".to_string()) {
        "../client/dist"
    } else {
        "./dist"
    };
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(pool.clone()))
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
                            .service(routes::instance::area),
                    )
                    .service(
                        web::scope("data")
                            .service(routes::raw_data::all)
                            .service(routes::raw_data::bound)
                            .service(routes::raw_data::area),
                    )
                    .service(
                        web::scope("v1").service(
                            web::scope("calc")
                                .service(routes::calculate::bootstrap)
                                .service(routes::calculate::cluster),
                        ),
                    ),
            )
            .service(
                Files::new("/", serve_from.to_string())
                    .index_file("index.html")
                    .prefer_utf8(true),
            )
    })
    .bind(("0.0.0.0", port.parse::<u16>().unwrap()))?
    .run()
    .await
}
