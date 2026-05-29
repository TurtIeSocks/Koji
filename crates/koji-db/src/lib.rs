//! koji-db — Koji's own sea-orm entities, queries, db enums + bridges, query
//! helpers, the `KojiDb` connection holder + bootstrap, and the db-centric
//! `ModelError`. Depends only on koji-core.

use geojson::{Feature, FeatureCollection};
use sea_orm::DatabaseConnection;
use serde::{Deserialize, Serialize};

pub mod db;
pub mod error;
pub mod utils;

pub use error::ModelError;

#[derive(Debug, Clone)]
pub struct KojiDb {
    pub koji: DatabaseConnection,
    pub scanner: DatabaseConnection,
}
