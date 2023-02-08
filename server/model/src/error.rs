use sea_orm::DbErr;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ModelError {
    #[error("Database Error: {0}")]
    Database(DbErr),
    #[error("Geojson Error: {0}")]
    Geojson(geojson::Error),
    #[error("[PROJECT]: {0}")]
    Project(String),
    #[error("[GEOFENCE]: {0}")]
    Geofence(String),
    #[error("[GEOFENCE_PROPERTY]: {0}")]
    GeofenceProperty(String),
    #[error("[GEOFENCE_PROJECT]: {0}")]
    GeofenceProject(String),
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
