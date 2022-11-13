use sea_orm::{DatabaseConnection, FromQueryResult};
use serde::{Deserialize, Serialize};

pub mod api;
pub mod scanner;

#[derive(Debug, Serialize, Deserialize)]
pub enum FloatType {
    F32(f32),
    F64(f64),
}

#[derive(Clone)]
pub struct KojiDb {
    pub data_db: DatabaseConnection,
    pub unown_db: Option<DatabaseConnection>,
}
