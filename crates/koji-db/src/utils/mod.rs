use super::*;

use std::env;

use log::LevelFilter;
use sea_orm::{ConnectOptions, Database, Order};

use crate::db::sea_orm_active_enums::{Category, Mode};

pub mod json;
pub mod sort;

pub use sort::json_related_sort;

// Boundary adapters: koji-core's mappers return the pure domain enums; the db
// layer needs the sea-orm enums. Convert across the boundary via `enum_bridge!`.

/// Map an inbound `mode` string (a legacy 12-value RDM value OR a canonical
/// 4-value string) to the storage [`Mode`]. `None`/unknown → `Mode::Unset`.
/// Goes through `koji_core::Mode::from_legacy` (the single source of truth for
/// lenient mode parsing) then bridges to the sea-orm enum.
pub fn get_enum(instance_type: Option<String>) -> Mode {
    instance_type
        .as_deref()
        .map(koji_core::Mode::from_legacy)
        .unwrap_or_default()
        .into()
}

/// Map an inbound `category` string to the storage [`Category`]. Matches the
/// lowercased string to a known variant; anything unrecognized → `String`.
/// (The match builds a domain `crate::category::Category`, which then bridges to
/// the sea-orm enum via `.into()`.)
pub fn get_category_enum(category: String) -> Category {
    use crate::category::Category as DomainCategory;
    // `Category` derives StrEnum (case-insensitive); unknown → `String`,
    // matching the prior manual fallthrough.
    DomainCategory::from_str_opt(&category)
        .unwrap_or(DomainCategory::String)
        .into()
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

pub(crate) fn parse_order(order_by: &str) -> Order {
    if order_by.to_lowercase().eq("asc") {
        Order::Asc
    } else {
        Order::Desc
    }
}
