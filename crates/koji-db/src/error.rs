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
    #[error("[PROPERTY]: {0}")]
    Property(String),
    #[error("[GEOFENCE]: {0}")]
    Geofence(String),
    #[error("[GEOFENCE_PROJECT]: {0}")]
    GeofenceProject(String),
    #[error("[GEOFENCE_PROPERTY]: {0}")]
    GeofenceProperty(String),
    #[error("[ROUTE]: {0}")]
    Route(String),
    #[error("[TileServer]: {0}")]
    TileServer(String),
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

#[cfg(test)]
mod tests {
    use super::*;

    // ── Display format ──────────────────────────────────────────────────────────

    #[test]
    fn display_project() {
        let e = ModelError::Project("no name".to_string());
        assert_eq!(format!("{e}"), "[PROJECT]: no name");
    }

    #[test]
    fn display_property() {
        let e = ModelError::Property("bad category".to_string());
        assert_eq!(format!("{e}"), "[PROPERTY]: bad category");
    }

    #[test]
    fn display_geofence() {
        let e = ModelError::Geofence("does not exist".to_string());
        assert_eq!(format!("{e}"), "[GEOFENCE]: does not exist");
    }

    #[test]
    fn display_geofence_project() {
        let e = ModelError::GeofenceProject("missing project_id".to_string());
        assert_eq!(format!("{e}"), "[GEOFENCE_PROJECT]: missing project_id");
    }

    #[test]
    fn display_geofence_property() {
        let e = ModelError::GeofenceProperty("missing geofence_id".to_string());
        assert_eq!(format!("{e}"), "[GEOFENCE_PROPERTY]: missing geofence_id");
    }

    #[test]
    fn display_route() {
        let e = ModelError::Route("no geometry".to_string());
        assert_eq!(format!("{e}"), "[ROUTE]: no geometry");
    }

    #[test]
    fn display_tile_server() {
        let e = ModelError::TileServer("no url".to_string());
        assert_eq!(format!("{e}"), "[TileServer]: no url");
    }

    #[test]
    fn display_custom() {
        let e = ModelError::Custom("arbitrary".to_string());
        assert_eq!(format!("{e}"), "arbitrary");
    }

    // ── From<DbErr> ─────────────────────────────────────────────────────────────

    #[test]
    fn from_dberr_wraps_in_database_variant() {
        let db_err = sea_orm::DbErr::Custom("conn failed".to_string());
        let model_err = ModelError::from(db_err);
        assert!(matches!(model_err, ModelError::Database(_)));
        assert!(format!("{model_err}").contains("Database Error"));
    }

    // ── From<geojson::Error> ────────────────────────────────────────────────────

    #[test]
    fn from_geojson_error_wraps_in_geojson_variant() {
        // Force a geojson parse error by trying to build a Geometry from a string.
        let bad = serde_json::json!("not a geometry");
        let result = geojson::Geometry::from_json_value(bad);
        let gj_err = result.unwrap_err();
        let model_err = ModelError::from(gj_err);
        assert!(matches!(model_err, ModelError::Geojson(_)));
        assert!(format!("{model_err}").contains("Geojson Error"));
    }
}
