use super::*;

use std::env;

use geojson::Value;
use log::LevelFilter;
use sea_orm::{ConnectOptions, Database, Order};

use crate::db::sea_orm_active_enums::{Category, Type};

pub mod json;

// Boundary adapters: koji-core's mappers return the pure domain enums; the db
// layer needs the sea-orm enums. Convert across the boundary via `enum_bridge!`.

pub fn get_enum(instance_type: Option<String>) -> Type {
    koji_core::get_enum(instance_type).into()
}

pub fn get_enum_by_geometry(enum_val: &Value) -> Type {
    koji_core::get_enum_by_geometry(enum_val).into()
}

pub fn get_category_enum(category: String) -> Category {
    koji_core::get_category_enum(category).into()
}

pub async fn get_database_struct() -> KojiDb {
    let koji_db_url = env::var("KOJI_DB_URL").expect("Need KOJI_DB_URL env var to run migrations");

    let scanner_db_url = env::var("SCANNER_DB_URL").expect("Need SCANNER_DB_URL env var");

    let max_connections: u32 = env::var("MAX_CONNECTIONS")
        .unwrap_or("100".to_string())
        .parse()
        .unwrap_or(100);

    let log_level = match std::env::var("LOG_LEVEL")
        .unwrap_or("info".to_string())
        .as_str()
    {
        "error" => LevelFilter::Error,
        "warn" => LevelFilter::Warn,
        "info" => LevelFilter::Info,
        "debug" => LevelFilter::Debug,
        "trace" => LevelFilter::Trace,
        _ => LevelFilter::Info,
    };
    let enable_logging = log_level == LevelFilter::Trace || log_level == LevelFilter::Debug;

    KojiDb {
        scanner: {
            let mut opt = ConnectOptions::new(scanner_db_url);
            opt.max_connections(max_connections);
            opt.sqlx_logging_level(log_level);
            opt.sqlx_logging(enable_logging);
            match Database::connect(opt).await {
                Ok(db) => db,
                Err(err) => panic!("Cannot connect to Scanner DB: {}", err),
            }
        },
        koji: {
            let mut opt = ConnectOptions::new(koji_db_url);
            opt.max_connections(max_connections);
            opt.sqlx_logging_level(log_level);
            opt.sqlx_logging(enable_logging);
            match Database::connect(opt).await {
                Ok(db) => db,
                Err(err) => panic!("Cannot connect to Kōji DB: {}", err),
            }
        },
    }
}

pub fn parse_order(order_by: &String) -> Order {
    if order_by.to_lowercase().eq("asc") {
        Order::Asc
    } else {
        Order::Desc
    }
}
