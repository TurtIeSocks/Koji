use geojson::{Feature, FeatureCollection};
use log;
use num_traits::Float;
use sea_orm::DatabaseConnection;
use serde::{Deserialize, Serialize};

pub mod api;
pub mod db;
pub mod error;
pub mod utils;

#[derive(Debug, Clone)]
pub struct KojiDb {
    pub koji_db: DatabaseConnection,
    pub data_db: DatabaseConnection,
    pub unown_db: Option<DatabaseConnection>,
}
