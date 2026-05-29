use super::*;

use std::env;

use geojson::Value;
use log::LevelFilter;
use sea_orm::{ConnectOptions, ConnectionTrait, Database, Order, Statement};

use crate::db::sea_orm_active_enums::{Category, Type};

pub mod json;
pub mod normalize;

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

    let scanner_db_url = if env::var("DATABASE_URL").is_ok() {
        log::warn!("[WARNING] `DATABASE_URL` is deprecated in favor of `SCANNER_DB_URL`");
        env::var("DATABASE_URL")
    } else {
        env::var("SCANNER_DB_URL")
    }
    .expect("Need SCANNER_DB_URL env var");

    let controller_db_url = if let Ok(var) = env::var("CONTROLLER_DB_URL") {
        var
    } else if let Ok(var) = env::var("UNOWN_DB_URL") {
        log::warn!("`UNOWN_DB_URL` is deprecated in favor of `CONTROLLER_DB_URL`");
        var
    } else if let Ok(var) = env::var("UNOWN_DB") {
        log::warn!("`UNOWN_DB` is deprecated in favor of `CONTROLLER_DB_URL`");
        var
    } else {
        "".to_string()
    };

    let max_connections: u32 = if let Ok(parsed) = env::var("MAX_CONNECTIONS")
        .unwrap_or("100".to_string())
        .parse()
    {
        parsed
    } else {
        100
    };

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

    let controller_connection = {
        let url = if controller_db_url.is_empty() {
            scanner_db_url.clone()
        } else {
            controller_db_url.clone()
        };
        let mut opt = ConnectOptions::new(url);
        opt.max_connections(max_connections);
        opt.sqlx_logging_level(log_level);
        opt.sqlx_logging(enable_logging);
        match Database::connect(opt).await {
            Ok(db) => db,
            Err(err) => panic!("Cannot connect to Controller DB: {}", err),
        }
    };

    let scanner_type = if controller_db_url.is_empty() {
        ScannerType::RDM
    } else {
        if controller_connection
            .execute(Statement::from_string(
                controller_connection.get_database_backend(),
                r#"SELECT name FROM `instance` LIMIT 1"#,
            ))
            .await
            .is_ok()
        {
            ScannerType::Hybrid
        } else {
            ScannerType::Unown
        }
    };

    log::info!("Determined Scanner Type: {}", scanner_type);

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
        controller: controller_connection,
        scanner_type,
    }
}

pub fn parse_order(order_by: &String) -> Order {
    if order_by.to_lowercase().eq("asc") {
        Order::Asc
    } else {
        Order::Desc
    }
}
