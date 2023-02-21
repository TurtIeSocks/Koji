use std::str::FromStr;

use geojson::{GeoJson, Geometry};
use sea_orm::Set;
use serde_json::Value;

use crate::{
    db::{geofence, geofence_project, geofence_property, project, sea_orm_active_enums::Category},
    error::ModelError,
};

use super::get_enum;

pub trait JsonToModel {
    fn to_geofence(&self) -> Result<geofence::ActiveModel, ModelError>;
    fn to_project(&self) -> Result<project::ActiveModel, ModelError>;
    fn to_geofence_property(
        &self,
        geofence_id: Option<u32>,
    ) -> Result<geofence_property::ActiveModel, ModelError>;
    fn to_geofence_project(&self) -> Result<geofence_project::ActiveModel, ModelError>;
}

impl JsonToModel for Value {
    fn to_geofence(&self) -> Result<geofence::ActiveModel, ModelError> {
        if let Some(incoming) = self.as_object() {
            let name = if let Some(name) = incoming.get("name") {
                name.as_str()
            } else {
                None
            };
            if let Some(name) = name {
                if let Some(geometry) = incoming.get("geometry") {
                    match Geometry::from_json_value(geometry.to_owned()) {
                        Ok(geometry) => {
                            let value = GeoJson::Geometry(geometry).to_json_value();
                            let mode = if let Some(mode) = incoming.get("mode") {
                                Some(mode.as_str().unwrap_or("unset").to_string())
                            } else {
                                None
                            };
                            let mode = get_enum(mode);
                            Ok(geofence::ActiveModel {
                                name: Set(name.to_string()),
                                geometry: Set(value),
                                mode: Set(mode),
                                ..Default::default()
                            })
                        }
                        Err(err) => Err(ModelError::Geofence(format!(
                            "geometry is invalid: {:?}",
                            err
                        ))),
                    }
                } else {
                    Err(ModelError::Geofence(format!(
                        "model does not have a geometry object: {:?}",
                        self
                    )))
                }
            } else {
                Err(ModelError::Geofence(format!(
                    "model does not have a name property: {:?}",
                    self
                )))
            }
        } else {
            Err(ModelError::Geofence(format!(
                "model is not an object: {:?}",
                self
            )))
        }
    }

    fn to_geofence_project(&self) -> Result<geofence_project::ActiveModel, ModelError> {
        if let Some(object) = self.as_object() {
            let geofence_id = if let Some(id) = self.get("geofence_id") {
                id.as_u64()
            } else {
                None
            };
            if let Some(geofence_id) = geofence_id {
                let project_id = if let Some(id) = self.get("project_id") {
                    id.as_u64()
                } else {
                    None
                };
                if let Some(project_id) = project_id {
                    Ok(geofence_project::ActiveModel {
                        project_id: Set(project_id as u32),
                        geofence_id: Set(geofence_id as u32),
                        ..Default::default()
                    })
                } else {
                    Err(ModelError::GeofenceProject(format!(
                        "project_id not found: {:?}",
                        object
                    )))
                }
            } else {
                Err(ModelError::GeofenceProject(format!(
                    "geofence_id not found: {:?}",
                    object
                )))
            }
        } else {
            Err(ModelError::GeofenceProject(format!(
                "invalid object {:?}",
                self
            )))
        }
    }

    fn to_geofence_property(
        &self,
        geofence_id: Option<u32>,
    ) -> Result<geofence_property::ActiveModel, ModelError> {
        if let Some(object) = self.as_object() {
            let geofence_id = if let Some(geofence_id) = geofence_id {
                Some(geofence_id as u64)
            } else if let Some(id) = self.get("geofence_id") {
                id.as_u64()
            } else {
                None
            };
            if let Some(geofence_id) = geofence_id {
                let property_id = if let Some(id) = self.get("property_id") {
                    id.as_u64()
                } else {
                    None
                };
                if let Some(property_id) = property_id {
                    let value = if let Some(value) = self.get("value") {
                        if let Some(value) = value.as_str() {
                            if value.len() > 0 {
                                Some(value.to_string())
                            } else {
                                None
                            }
                        } else if value == &Value::Null {
                            None
                        } else {
                            Some(value.to_string())
                        }
                    } else {
                        None
                    };
                    Ok(geofence_property::ActiveModel {
                        property_id: Set(property_id as u32),
                        geofence_id: Set(geofence_id as u32),
                        value: Set(value),
                        ..Default::default()
                    })
                } else {
                    Err(ModelError::GeofenceProperty(format!(
                        "property_id not found: {:?}",
                        object
                    )))
                }
            } else {
                Err(ModelError::GeofenceProperty(format!(
                    "geofence_id not found: {:?}",
                    object
                )))
            }
        } else {
            Err(ModelError::GeofenceProperty(format!(
                "invalid object {:?}",
                self
            )))
        }
    }

    fn to_project(&self) -> Result<project::ActiveModel, ModelError> {
        if let Some(incoming) = self.as_object() {
            let name = if let Some(name) = incoming.get("name") {
                name.as_str()
            } else {
                None
            };
            let scanner = if let Some(scanner) = incoming.get("scanner") {
                scanner.as_bool().unwrap_or(false)
            } else {
                false
            };
            if let Some(name) = name {
                let api_endpoint = if let Some(api_endpoint) = incoming.get("api_endpoint") {
                    match api_endpoint.as_str() {
                        Some(api_endpoint) => Some(api_endpoint.to_string()),
                        None => None,
                    }
                } else {
                    None
                };
                let api_key = if let Some(api_key) = incoming.get("api_key") {
                    match api_key.as_str() {
                        Some(api_key) => Some(api_key.to_string()),
                        None => None,
                    }
                } else {
                    None
                };
                Ok(project::ActiveModel {
                    name: Set(name.to_string()),
                    scanner: Set(scanner),
                    api_endpoint: Set(api_endpoint),
                    api_key: Set(api_key),
                    ..Default::default()
                })
            } else {
                Err(ModelError::Project(format!(
                    "model does not have a name property: {:?}",
                    self
                )))
            }
        } else {
            Err(ModelError::Project(format!(
                "model is not an object: {:?}",
                self
            )))
        }
    }
}

pub fn parse_property_value(value: &String, category: &Category) -> Value {
    match category {
        Category::String | Category::Color => serde_json::Value::String(value.to_string()),
        Category::Number => serde_json::Value::Number(
            serde_json::Number::from_f64(value.parse::<f64>().unwrap_or(0.)).unwrap(),
        ),
        Category::Boolean => serde_json::Value::Bool(value.parse::<bool>().unwrap_or(false)),
        Category::Object => serde_json::Value::from_str(&value).unwrap(),
        Category::Array => serde_json::Value::from_str(&value).unwrap(),
        Category::Database => serde_json::Value::Null,
    }
}
