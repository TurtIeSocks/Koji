use actix_files::Files;
use actix_web::{middleware, web, App, HttpServer};
use sea_orm::{Database, DatabaseConnection, DbErr};

mod entities;
mod models;
mod queries;
mod routes;
mod utils;

async fn set_up_db() -> Result<DatabaseConnection, DbErr> {
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL");
    Ok(Database::connect(database_url).await?)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    let db = match set_up_db().await {
        Ok(db) => db,
        Err(err) => panic!("{}", err),
    };
    let scanner_type = std::env::var("SCANNER_TYPE").unwrap_or("rdm".to_string());

    let port = std::env::var("PORT").unwrap_or("8080".to_string());
    let serve_from = if std::env::var("NODE_ENV") == Ok("development".to_string()) {
        "../client/dist"
    } else {
        "./dist"
    };
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(db.clone()))
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
