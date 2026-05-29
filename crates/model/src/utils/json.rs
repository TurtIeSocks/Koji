use std::{collections::HashMap, str::FromStr};

use geojson::{GeoJson, Geometry};
use sea_orm::Set;
use serde_json::Value;

use crate::{
    db::{
        geofence, geofence_project, geofence_property, project, property, route,
        sea_orm_active_enums::Category, tile_server,
    },
    error::ModelError,
};

use super::{get_category_enum, get_enum};

pub trait JsonToModel {
    fn to_geofence(&self) -> Result<geofence::ActiveModel, ModelError>;
    fn to_project(&self) -> Result<project::ActiveModel, ModelError>;
    fn to_geofence_property(
        &self,
        geofence_id: Option<u32>,
    ) -> Result<geofence_property::ActiveModel, ModelError>;
    fn to_geofence_project(&self) -> Result<geofence_project::ActiveModel, ModelError>;
    fn to_property(&self) -> Result<property::ActiveModel, ModelError>;
    fn to_route(&self) -> Result<route::ActiveModel, ModelError>;
    fn to_tileserver(&self) -> Result<tile_server::ActiveModel, ModelError>;
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
                            let parent = if let Some(parent) = incoming.get("parent") {
                                if let Some(parent) = parent.as_u64() {
                                    Some(parent as u32)
                                } else {
                                    None
                                }
                            } else {
                                None
                            };
                            let mode = get_enum(mode);
                            Ok(geofence::ActiveModel {
                                name: Set(name.to_string()),
                                geometry: Set(value),
                                parent: Set(parent),
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
                let description = if let Some(description) = incoming.get("description") {
                    match description.as_str() {
                        Some(description) => Some(description.to_string()),
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
                    description: Set(description),
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

    fn to_property(&self) -> Result<property::ActiveModel, ModelError> {
        if let Some(incoming) = self.as_object() {
            let name = if let Some(name) = incoming.get("name") {
                name.as_str()
            } else {
                None
            };
            let category = if let Some(category) = incoming.get("category") {
                if let Some(category) = category.as_str() {
                    Some(get_category_enum(category.to_string()))
                } else {
                    None
                }
            } else {
                None
            };
            let mut default_value = if let Some(default_value) = incoming.get("default_value") {
                if let Some(default_value) = default_value.as_str() {
                    Some(default_value.to_string())
                } else {
                    Some(default_value.to_string())
                }
            } else {
                None
            };
            if let Some(value_check) = default_value.as_ref() {
                if value_check == "null" {
                    default_value = None;
                }
            }
            if let Some(name) = name {
                if let Some(category) = category {
                    Ok(property::ActiveModel {
                        name: Set(name.to_string()),
                        category: Set(category),
                        default_value: Set(default_value),
                        ..Default::default()
                    })
                } else {
                    Err(ModelError::Property(format!(
                        "model does not have a category property: {:?}",
                        self
                    )))
                }
            } else {
                Err(ModelError::Property(format!(
                    "model does not have a name property: {:?}",
                    self
                )))
            }
        } else {
            Err(ModelError::Property(format!(
                "model is not an object: {:?}",
                self
            )))
        }
    }

    fn to_route(&self) -> Result<route::ActiveModel, ModelError> {
        if let Some(incoming) = self.as_object() {
            let name = if let Some(name) = incoming.get("name") {
                name.as_str()
            } else {
                None
            };
            let geofence_id = if let Some(geofence_id) = incoming.get("geofence_id") {
                geofence_id.as_u64()
            } else {
                None
            };
            if let Some(name) = name {
                if let Some(geofence_id) = geofence_id {
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
                                let description =
                                    if let Some(description) = incoming.get("description") {
                                        if let Some(description) = description.as_str() {
                                            Some(description.to_string())
                                        } else {
                                            None
                                        }
                                    } else {
                                        None
                                    };
                                Ok(route::ActiveModel {
                                    name: Set(name.to_string()),
                                    geometry: Set(value),
                                    mode: Set(mode),
                                    geofence_id: Set(geofence_id as u32),
                                    description: Set(description),
                                    ..Default::default()
                                })
                            }
                            Err(err) => {
                                Err(ModelError::Route(format!("geometry is invalid: {:?}", err)))
                            }
                        }
                    } else {
                        Err(ModelError::Route(format!(
                            "model does not have a geometry object: {:?}",
                            self
                        )))
                    }
                } else {
                    Err(ModelError::Route(format!(
                        "model does not have a geofence_id property: {:?}",
                        self
                    )))
                }
            } else {
                Err(ModelError::Route(format!(
                    "model does not have a name property: {:?}",
                    self
                )))
            }
        } else {
            Err(ModelError::Route(format!(
                "model is not an object: {:?}",
                self
            )))
        }
    }

    fn to_tileserver(&self) -> Result<tile_server::ActiveModel, ModelError> {
        if let Some(incoming) = self.as_object() {
            let name = if let Some(name) = incoming.get("name") {
                name.as_str()
            } else {
                None
            };
            let url = if let Some(url) = incoming.get("url") {
                url.as_str()
            } else {
                None
            };
            if let Some(name) = name {
                if let Some(url) = url {
                    Ok(tile_server::ActiveModel {
                        name: Set(name.to_string()),
                        url: Set(url.to_string()),
                        ..Default::default()
                    })
                } else {
                    Err(ModelError::TileServer(format!(
                        "model does not have a url property: {:?}",
                        self
                    )))
                }
            } else {
                Err(ModelError::TileServer(format!(
                    "model does not have a name property: {:?}",
                    self
                )))
            }
        } else {
            Err(ModelError::TileServer(format!(
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

pub fn determine_category_by_value(
    key: &str,
    value: Value,
    db_json: &HashMap<&str, Value>,
) -> (Category, Option<Value>) {
    let mut actual_value: Option<Value> = Some(value.clone().into());
    let mut category = Category::String;

    if db_json.contains_key(key) {
        category = Category::Database;
        actual_value = None;
    } else if let Some(val) = value.as_bool() {
        category = Category::Boolean;
        actual_value = Some(val.into());
    } else if let Some(val) = value.as_f64() {
        category = Category::Number;
        actual_value = Some(val.into());
    } else if let Some(_) = value.as_array() {
        category = Category::Array;
    } else if let Some(_) = value.as_object() {
        category = Category::Object;
    } else if let Some(value) = value.as_str() {
        match value.parse::<f64>() {
            Ok(val) => {
                category = Category::Number;
                actual_value = Some(val.into());
            }
            Err(_) => {
                if value == "true" {
                    category = Category::Boolean;
                    actual_value = Some(true.into());
                } else if value == "false" {
                    category = Category::Boolean;
                    actual_value = Some(false.into());
                } else if value.starts_with("#") || value.starts_with("rgb") {
                    category = Category::Color;
                    actual_value = Some(value.to_string().into());
                } else if value.starts_with("{") {
                    category = Category::Object;
                    actual_value = match serde_json::from_str::<Value>(value) {
                        Ok(val) => Some(val.into()),
                        Err(_) => None,
                    };
                } else if value.starts_with("[") {
                    category = Category::Array;
                    actual_value = match serde_json::from_str::<Value>(value) {
                        Ok(val) => Some(val.into()),
                        Err(_) => None,
                    };
                } else {
                    actual_value = Some(value.to_string().into());
                }
            }
        };
    }
    (category, actual_value)
}
