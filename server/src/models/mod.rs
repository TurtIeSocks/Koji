use sea_orm::{DatabaseConnection, FromQueryResult};
use serde::{Deserialize, Serialize};

pub mod api;
pub mod scanner;

#[derive(Clone)]
pub struct KojiDb {
    pub data_db: DatabaseConnection,
    pub unown_db: Option<DatabaseConnection>,
}
