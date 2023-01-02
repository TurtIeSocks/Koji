use geojson::{Feature, FeatureCollection};
use num_traits::Float;
use sea_orm::DatabaseConnection;
use serde::{Deserialize, Serialize};

pub mod api;
pub mod db;
pub mod utils;

#[derive(Clone)]
pub struct KojiDb {
    pub koji_db: DatabaseConnection,
    pub data_db: DatabaseConnection,
    pub unown_db: Option<DatabaseConnection>,
}
