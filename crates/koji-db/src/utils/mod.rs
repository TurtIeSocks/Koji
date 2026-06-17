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

    let golbat_db_url = env::var("GOLBAT_DB_URL").expect("Need GOLBAT_DB_URL env var");

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
        golbat: {
            let mut opt = ConnectOptions::new(golbat_db_url);
            opt.max_connections(max_connections);
            opt.sqlx_logging_level(log_level);
            opt.sqlx_logging(enable_logging);
            match Database::connect(opt).await {
                Ok(db) => db,
                Err(err) => panic!("Cannot connect to Golbat DB: {}", err),
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

#[cfg(test)]
mod tests {
    use super::*;

    // ── parse_order ─────────────────────────────────────────────────────────────

    #[test]
    fn parse_order_asc_lowercase() {
        assert!(matches!(parse_order("asc"), Order::Asc));
    }

    #[test]
    fn parse_order_asc_uppercase() {
        assert!(matches!(parse_order("ASC"), Order::Asc));
    }

    #[test]
    fn parse_order_asc_mixed_case() {
        assert!(matches!(parse_order("Asc"), Order::Asc));
    }

    #[test]
    fn parse_order_desc_lowercase() {
        assert!(matches!(parse_order("desc"), Order::Desc));
    }

    #[test]
    fn parse_order_desc_uppercase() {
        assert!(matches!(parse_order("DESC"), Order::Desc));
    }

    #[test]
    fn parse_order_empty_string_defaults_to_desc() {
        assert!(matches!(parse_order(""), Order::Desc));
    }

    #[test]
    fn parse_order_unrecognized_defaults_to_desc() {
        assert!(matches!(parse_order("random"), Order::Desc));
    }

    // ── get_enum (mode) ─────────────────────────────────────────────────────────

    #[test]
    fn get_enum_none_is_unset() {
        use crate::db::sea_orm_active_enums::Mode;
        assert_eq!(get_enum(None), Mode::Unset);
    }

    #[test]
    fn get_enum_pokemon_string() {
        use crate::db::sea_orm_active_enums::Mode;
        assert_eq!(get_enum(Some("pokemon".to_string())), Mode::Pokemon);
    }

    #[test]
    fn get_enum_fort_string() {
        use crate::db::sea_orm_active_enums::Mode;
        assert_eq!(get_enum(Some("fort".to_string())), Mode::Fort);
    }

    #[test]
    fn get_enum_quest_string() {
        use crate::db::sea_orm_active_enums::Mode;
        assert_eq!(get_enum(Some("quest".to_string())), Mode::Quest);
    }

    #[test]
    fn get_enum_circle_pokemon_legacy_maps_to_pokemon() {
        // Legacy 12-value RDM string → canonical 4-value Mode via from_legacy.
        use crate::db::sea_orm_active_enums::Mode;
        assert_eq!(get_enum(Some("circle_pokemon".to_string())), Mode::Pokemon);
    }

    #[test]
    fn get_enum_circle_fort_legacy_maps_to_fort() {
        use crate::db::sea_orm_active_enums::Mode;
        assert_eq!(get_enum(Some("circle_raid".to_string())), Mode::Fort);
    }

    #[test]
    fn get_enum_unknown_string_maps_to_unset() {
        use crate::db::sea_orm_active_enums::Mode;
        assert_eq!(
            get_enum(Some("completely_unknown".to_string())),
            Mode::Unset
        );
    }

    // ── get_category_enum ───────────────────────────────────────────────────────

    #[test]
    fn get_category_string() {
        use crate::db::sea_orm_active_enums::Category;
        assert_eq!(get_category_enum("string".to_string()), Category::String);
    }

    #[test]
    fn get_category_boolean() {
        use crate::db::sea_orm_active_enums::Category;
        assert_eq!(get_category_enum("boolean".to_string()), Category::Boolean);
    }

    #[test]
    fn get_category_number() {
        use crate::db::sea_orm_active_enums::Category;
        assert_eq!(get_category_enum("number".to_string()), Category::Number);
    }

    #[test]
    fn get_category_object() {
        use crate::db::sea_orm_active_enums::Category;
        assert_eq!(get_category_enum("object".to_string()), Category::Object);
    }

    #[test]
    fn get_category_array() {
        use crate::db::sea_orm_active_enums::Category;
        assert_eq!(get_category_enum("array".to_string()), Category::Array);
    }

    #[test]
    fn get_category_database() {
        use crate::db::sea_orm_active_enums::Category;
        assert_eq!(
            get_category_enum("database".to_string()),
            Category::Database
        );
    }

    #[test]
    fn get_category_color() {
        use crate::db::sea_orm_active_enums::Category;
        assert_eq!(get_category_enum("color".to_string()), Category::Color);
    }

    #[test]
    fn get_category_uppercase_works() {
        // StrEnum is case-insensitive.
        use crate::db::sea_orm_active_enums::Category;
        assert_eq!(get_category_enum("STRING".to_string()), Category::String);
    }

    #[test]
    fn get_category_unknown_falls_back_to_string() {
        use crate::db::sea_orm_active_enums::Category;
        assert_eq!(
            get_category_enum("totally_unknown".to_string()),
            Category::String
        );
    }
}
