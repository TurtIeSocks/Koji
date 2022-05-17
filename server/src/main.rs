#[macro_use]
extern crate diesel;

use actix_files::Files;
use actix_web::{middleware, web, App, HttpServer};
use diesel::prelude::*;
use diesel::r2d2::{self, ConnectionManager};

pub type DbPool = r2d2::Pool<ConnectionManager<MysqlConnection>>;

mod handlers;
mod marker_gen;
mod models;
mod queries;
mod schema;
mod sql_types;

#[actix_web::main] // or #[tokio::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();

    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL");
    let manager = ConnectionManager::<MysqlConnection>::new(database_url);
    let pool: DbPool = r2d2::Pool::builder()
        .build(manager)
        .expect("Failed to create pool.");
    let serve_from = if std::env::var("NODE_ENV") == Ok("development".to_string()) {
        "../dist"
    } else {
        "./dist"
    };

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .wrap(middleware::Logger::new("%s | %r - %b bytes in %D ms (%a)"))
            .service(handlers::config)
            .service(handlers::spawnpoints)
            .service(handlers::all_spawnpoints)
            .service(handlers::gyms)
            .service(handlers::pokestops)
            .service(handlers::instances)
            .service(
                Files::new("/", serve_from.to_string())
                    .index_file("index.html")
                    .prefer_utf8(true),
            )
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}
