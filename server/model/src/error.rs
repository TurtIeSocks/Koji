use sea_orm::DbErr;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ModelError {
    #[error("Database Error: {0}")]
    Database(DbErr),
    #[error("Geojson Error: {0}")]
    Geojson(geojson::Error),
    #[error("Not Implemented: {0}")]
    NotImplemented(String),
    #[error("{0}")]
    Custom(String),
}

impl From<DbErr> for ModelError {
    fn from(error: DbErr) -> Self {
        Self::Database(error)
    }
}

impl From<geojson::Error> for ModelError {
    fn from(error: geojson::Error) -> Self {
        Self::Geojson(error)
    }
}
