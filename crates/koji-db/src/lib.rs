//! koji-db — Koji's own sea-orm entities, queries, db enums + bridges, query
//! helpers, the `KojiDb` connection holder + bootstrap, and the db-centric
//! `ModelError`. Depends only on koji-core.

use geojson::{Feature, FeatureCollection};
use sea_orm::DatabaseConnection;
use serde::{Deserialize, Serialize};

pub mod category;
pub mod db;
pub mod error;
pub mod name_modifier;
pub mod query_args;
pub mod utils;

pub use category::Category;
pub use error::ModelError;

/// Generates 1:1 `From` impls both directions between two enums whose variant
/// idents match exactly. Invoke from a crate that owns at least one of the two
/// enums (orphan rule).
///
/// ```ignore
/// enum_bridge!(db::Category, crate::category::Category, [Boolean, String, ...]);
/// ```
#[macro_export]
macro_rules! enum_bridge {
    ($a:ty, $b:ty, [$($variant:ident),+ $(,)?]) => {
        impl ::core::convert::From<$a> for $b {
            fn from(value: $a) -> Self {
                match value { $(<$a>::$variant => <$b>::$variant,)+ }
            }
        }
        impl ::core::convert::From<$b> for $a {
            fn from(value: $b) -> Self {
                match value { $(<$b>::$variant => <$a>::$variant,)+ }
            }
        }
    };
}

#[derive(Debug, Clone)]
pub struct KojiDb {
    pub koji: DatabaseConnection,
    pub scanner: DatabaseConnection,
}
