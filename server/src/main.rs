#[macro_use]
extern crate diesel;

use actix_files::Files;
use actix_web::{middleware, web, App, HttpServer};
use diesel::prelude::*;
use diesel::r2d2::{self, ConnectionManager};

pub type DbPool = r2d2::Pool<ConnectionManager<MysqlConnection>>;

mod handlers;
mod models;
mod schema;

#[actix_web::main] // or #[tokio::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();

    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    // set up database connection pool
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL");
    let manager = ConnectionManager::<MysqlConnection>::new(database_url);
    let pool: DbPool = r2d2::Pool::builder()
        .build(manager)
        .expect("Failed to create pool.");

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .wrap(middleware::Logger::default())
            .service(handlers::spawnpoints)
            .service(handlers::gyms)
            .service(handlers::pokestops)
            .service(
                Files::new("/", "../dist")
                    .index_file("index.html")
                    .prefer_utf8(true),
            )
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
