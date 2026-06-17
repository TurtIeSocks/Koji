use std::{collections::HashMap, str::FromStr};
use koji_core::Precision;

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

#[allow(clippy::result_large_err)]
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
            let name = incoming.get("name").and_then(|v| v.as_str());
            if let Some(name) = name {
                if let Some(geometry) = incoming.get("geometry") {
                    match Geometry::from_json_value(geometry.to_owned()) {
                        Ok(geometry) => {
                            let value = GeoJson::Geometry(geometry).to_json_value();
                            let mode = incoming
                                .get("mode")
                                .map(|mode| mode.as_str().unwrap_or("unset").to_string());
                            let parent = incoming
                                .get("parent")
                                .and_then(|v| v.as_u64())
                                .map(|parent| parent as u32);
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
            let geofence_id = self.get("geofence_id").and_then(|v| v.as_u64());
            if let Some(geofence_id) = geofence_id {
                let project_id = self.get("project_id").and_then(|v| v.as_u64());
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
            } else {
                self.get("geofence_id").and_then(|v| v.as_u64())
            };
            if let Some(geofence_id) = geofence_id {
                let property_id = self.get("property_id").and_then(|v| v.as_u64());
                if let Some(property_id) = property_id {
                    let value = if let Some(value) = self.get("value") {
                        if let Some(value) = value.as_str() {
                            if !value.is_empty() {
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
            let name = incoming.get("name").and_then(|v| v.as_str());
            let golbat = incoming
                .get("golbat")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            if let Some(name) = name {
                let api_endpoint = incoming
                    .get("api_endpoint")
                    .and_then(|v| v.as_str())
                    .map(|api_endpoint| api_endpoint.to_string());
                let api_key = incoming
                    .get("api_key")
                    .and_then(|v| v.as_str())
                    .map(|api_key| api_key.to_string());
                let description = incoming
                    .get("description")
                    .and_then(|v| v.as_str())
                    .map(|description| description.to_string());
                Ok(project::ActiveModel {
                    name: Set(name.to_string()),
                    golbat: Set(golbat),
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
            let name = incoming.get("name").and_then(|v| v.as_str());
            let category = incoming
                .get("category")
                .and_then(|v| v.as_str())
                .map(|category| get_category_enum(category.to_string()));
            let mut default_value = if let Some(default_value) = incoming.get("default_value") {
                if let Some(default_value) = default_value.as_str() {
                    Some(default_value.to_string())
                } else {
                    Some(default_value.to_string())
                }
            } else {
                None
            };
            if let Some(value_check) = default_value.as_ref()
                && value_check == "null"
            {
                default_value = None;
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
            let name = incoming.get("name").and_then(|v| v.as_str());
            let geofence_id = incoming.get("geofence_id").and_then(|v| v.as_u64());
            if let Some(name) = name {
                if let Some(geofence_id) = geofence_id {
                    if let Some(geometry) = incoming.get("geometry") {
                        match Geometry::from_json_value(geometry.to_owned()) {
                            Ok(geometry) => {
                                let value = GeoJson::Geometry(geometry).to_json_value();
                                let mode = incoming
                                    .get("mode")
                                    .map(|mode| mode.as_str().unwrap_or("unset").to_string());
                                let mode = get_enum(mode);
                                let description = incoming
                                    .get("description")
                                    .and_then(|v| v.as_str())
                                    .map(|description| description.to_string());
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
            let name = incoming.get("name").and_then(|v| v.as_str());
            let url = incoming.get("url").and_then(|v| v.as_str());
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
        Category::String | Category::Color => Value::String(value.to_string()),
        Category::Number => {
            // Unparseable values fall back to 0.0; NaN/inf can't be represented as a JSON
            // number, so they also fall back to a finite 0.0 (no panic on bad data). The
            // result is always a float, matching `from_f64`'s output for valid numbers.
            let parsed = value.parse::<Precision>().unwrap_or(0.0);
            let finite = if parsed.is_finite() { parsed } else { 0.0 };
            Value::Number(
                serde_json::Number::from_f64(finite).unwrap_or_else(|| serde_json::Number::from(0)),
            )
        }
        Category::Boolean => Value::Bool(value.parse::<bool>().unwrap_or(false)),
        // Malformed Object/Array JSON resolves to Null rather than crashing the response.
        Category::Object | Category::Array => Value::from_str(value).unwrap_or(Value::Null),
        Category::Database => Value::Null,
    }
}

/// Thin wrapper so tests can call determine_category_by_value with a
/// `HashMap<&str, Value>` built inline.
#[cfg(test)]
fn dcbv<'a>(key: &'a str, value: Value, db_keys: &[&'a str]) -> (Category, Option<Value>) {
    let mut map = HashMap::new();
    for k in db_keys {
        map.insert(*k, Value::Null);
    }
    determine_category_by_value(key, value, &map)
}

pub fn determine_category_by_value(
    key: &str,
    value: Value,
    db_json: &HashMap<&str, Value>,
) -> (Category, Option<Value>) {
    let mut actual_value: Option<Value> = Some(value.clone());
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
    } else if value.as_array().is_some() {
        category = Category::Array;
    } else if value.as_object().is_some() {
        category = Category::Object;
    } else if let Some(value) = value.as_str() {
        match value.parse::<Precision>() {
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
                    actual_value = serde_json::from_str::<Value>(value).ok();
                } else if value.starts_with("[") {
                    category = Category::Array;
                    actual_value = serde_json::from_str::<Value>(value).ok();
                } else {
                    actual_value = Some(value.to_string().into());
                }
            }
        };
    }
    (category, actual_value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // ── parse_property_value ────────────────────────────────────────────────────

    #[test]
    fn parse_string_category_returns_string() {
        let v = parse_property_value(&"hello".to_string(), &Category::String);
        assert_eq!(v, json!("hello"));
    }

    #[test]
    fn parse_color_category_returns_string() {
        let v = parse_property_value(&"#ff0000".to_string(), &Category::Color);
        assert_eq!(v, json!("#ff0000"));
    }

    #[test]
    fn parse_number_category_valid_float() {
        let v = parse_property_value(&"3.15".to_string(), &Category::Number);
        assert_eq!(v.as_f64().unwrap(), 3.15);
    }

    #[test]
    fn parse_number_category_invalid_falls_back_to_zero() {
        let v = parse_property_value(&"not-a-number".to_string(), &Category::Number);
        assert_eq!(v.as_f64().unwrap(), 0.0);
    }

    #[test]
    fn parse_boolean_true() {
        let v = parse_property_value(&"true".to_string(), &Category::Boolean);
        assert_eq!(v, json!(true));
    }

    #[test]
    fn parse_boolean_false() {
        let v = parse_property_value(&"false".to_string(), &Category::Boolean);
        assert_eq!(v, json!(false));
    }

    #[test]
    fn parse_boolean_invalid_falls_back_to_false() {
        let v = parse_property_value(&"yes".to_string(), &Category::Boolean);
        assert_eq!(v, json!(false));
    }

    #[test]
    fn parse_object_category() {
        let v = parse_property_value(&r#"{"key":"val"}"#.to_string(), &Category::Object);
        assert_eq!(v["key"], json!("val"));
    }

    #[test]
    fn parse_array_category() {
        let v = parse_property_value(&"[1,2,3]".to_string(), &Category::Array);
        assert_eq!(v, json!([1, 2, 3]));
    }

    #[test]
    fn parse_database_category_returns_null() {
        // Database category is a virtual marker — value is unused, always Null.
        let v = parse_property_value(&"anything".to_string(), &Category::Database);
        assert_eq!(v, json!(null));
    }

    #[test]
    fn parse_property_value_number_nan_falls_back_to_zero() {
        assert_eq!(
            parse_property_value(&"nan".to_string(), &Category::Number),
            serde_json::json!(0.0)
        );
    }
    #[test]
    fn parse_property_value_number_inf_falls_back_to_zero() {
        assert_eq!(
            parse_property_value(&"inf".to_string(), &Category::Number),
            serde_json::json!(0.0)
        );
    }
    #[test]
    fn parse_property_value_number_unparseable_falls_back_to_zero() {
        assert_eq!(
            parse_property_value(&"abc".to_string(), &Category::Number),
            serde_json::json!(0.0)
        );
    }
    #[test]
    fn parse_property_value_malformed_object_is_null() {
        assert_eq!(
            parse_property_value(&"{not json".to_string(), &Category::Object),
            serde_json::Value::Null
        );
    }
    #[test]
    fn parse_property_value_malformed_array_is_null() {
        assert_eq!(
            parse_property_value(&"[1,".to_string(), &Category::Array),
            serde_json::Value::Null
        );
    }
    #[test]
    fn parse_property_value_valid_object_round_trips() {
        assert_eq!(
            parse_property_value(&r#"{"a":1}"#.to_string(), &Category::Object),
            serde_json::json!({"a": 1})
        );
    }

    // ── determine_category_by_value ─────────────────────────────────────────────

    #[test]
    fn dcbv_db_key_overrides_everything() {
        // A key that appears in db_json → Database regardless of value type.
        let (cat, val) = dcbv("name", json!("Boulder"), &["name"]);
        assert_eq!(cat, Category::Database);
        assert_eq!(val, None);
    }

    #[test]
    fn dcbv_bool_true() {
        let (cat, val) = dcbv("active", json!(true), &[]);
        assert_eq!(cat, Category::Boolean);
        assert_eq!(val, Some(json!(true)));
    }

    #[test]
    fn dcbv_bool_false() {
        let (cat, val) = dcbv("active", json!(false), &[]);
        assert_eq!(cat, Category::Boolean);
        assert_eq!(val, Some(json!(false)));
    }

    #[test]
    fn dcbv_number_json() {
        let (cat, val) = dcbv("count", json!(42.0), &[]);
        assert_eq!(cat, Category::Number);
        assert_eq!(val.unwrap().as_f64().unwrap(), 42.0);
    }

    #[test]
    fn dcbv_array() {
        let (cat, val) = dcbv("tags", json!(["a", "b"]), &[]);
        assert_eq!(cat, Category::Array);
        assert!(val.is_some());
    }

    #[test]
    fn dcbv_object() {
        let (cat, val) = dcbv("meta", json!({"x": 1}), &[]);
        assert_eq!(cat, Category::Object);
        assert!(val.is_some());
    }

    #[test]
    fn dcbv_string_plain() {
        let (cat, val) = dcbv("label", json!("hello"), &[]);
        assert_eq!(cat, Category::String);
        assert_eq!(val, Some(json!("hello")));
    }

    #[test]
    fn dcbv_string_numeric_coerces_to_number() {
        let (cat, val) = dcbv("score", json!("7.5"), &[]);
        assert_eq!(cat, Category::Number);
        assert_eq!(val.unwrap().as_f64().unwrap(), 7.5);
    }

    #[test]
    fn dcbv_string_true_coerces_to_boolean() {
        let (cat, val) = dcbv("flag", json!("true"), &[]);
        assert_eq!(cat, Category::Boolean);
        assert_eq!(val, Some(json!(true)));
    }

    #[test]
    fn dcbv_string_false_coerces_to_boolean() {
        let (cat, val) = dcbv("flag", json!("false"), &[]);
        assert_eq!(cat, Category::Boolean);
        assert_eq!(val, Some(json!(false)));
    }

    #[test]
    fn dcbv_string_hash_color() {
        let (cat, val) = dcbv("color", json!("#aabbcc"), &[]);
        assert_eq!(cat, Category::Color);
        assert_eq!(val, Some(json!("#aabbcc")));
    }

    #[test]
    fn dcbv_string_rgb_color() {
        let (cat, val) = dcbv("color", json!("rgb(1,2,3)"), &[]);
        assert_eq!(cat, Category::Color);
        assert_eq!(val, Some(json!("rgb(1,2,3)")));
    }

    #[test]
    fn dcbv_string_json_object() {
        let (cat, val) = dcbv("meta", json!(r#"{"a":1}"#), &[]);
        assert_eq!(cat, Category::Object);
        assert_eq!(val.unwrap()["a"], 1);
    }

    #[test]
    fn dcbv_string_json_array() {
        let (cat, val) = dcbv("list", json!("[1,2]"), &[]);
        assert_eq!(cat, Category::Array);
        assert_eq!(val, Some(json!([1, 2])));
    }

    // ── JsonToModel — to_geofence ───────────────────────────────────────────────

    fn valid_geometry() -> serde_json::Value {
        json!({
            "type": "Polygon",
            "coordinates": [[[0.0,0.0],[1.0,0.0],[1.0,1.0],[0.0,1.0],[0.0,0.0]]]
        })
    }

    #[test]
    fn to_geofence_ok_with_name_and_geometry() {
        let v = json!({ "name": "TestFence", "geometry": valid_geometry() });
        let model = v.to_geofence().unwrap();
        // name round-trips via the ActiveModel Set wrapper.
        assert_eq!(model.name.unwrap(), "TestFence");
    }

    #[test]
    fn to_geofence_no_name_is_err() {
        let v = json!({ "geometry": valid_geometry() });
        assert!(v.to_geofence().is_err());
    }

    #[test]
    fn to_geofence_no_geometry_is_err() {
        let v = json!({ "name": "Fence" });
        assert!(v.to_geofence().is_err());
    }

    #[test]
    fn to_geofence_invalid_geometry_is_err() {
        let v = json!({ "name": "Fence", "geometry": "not geometry" });
        assert!(v.to_geofence().is_err());
    }

    #[test]
    fn to_geofence_not_object_is_err() {
        let v = json!("string");
        assert!(v.to_geofence().is_err());
    }

    #[test]
    fn to_geofence_with_mode_and_parent() {
        let v = json!({
            "name": "Child",
            "geometry": valid_geometry(),
            "mode": "pokemon",
            "parent": 5
        });
        let model = v.to_geofence().unwrap();
        assert_eq!(model.parent.unwrap(), Some(5u32));
    }

    #[test]
    fn to_geofence_with_invalid_mode_falls_back_to_unset() {
        // Unrecognized mode strings collapse to Mode::Unset via from_legacy.
        let v = json!({ "name": "Fence", "geometry": valid_geometry(), "mode": "nonsense" });
        // Should not error — just stores Unset.
        assert!(v.to_geofence().is_ok());
    }

    // ── JsonToModel — to_project ────────────────────────────────────────────────

    #[test]
    fn to_project_ok_minimal() {
        let v = json!({ "name": "MyProject" });
        let model = v.to_project().unwrap();
        assert_eq!(model.name.unwrap(), "MyProject");
        assert!(!model.golbat.unwrap());
    }

    #[test]
    fn to_project_ok_full() {
        let v = json!({
            "name": "Full",
            "golbat": true,
            "api_endpoint": "https://example.com",
            "api_key": "secret",
            "description": "A project"
        });
        let model = v.to_project().unwrap();
        assert_eq!(model.name.unwrap(), "Full");
        assert!(model.golbat.unwrap());
        assert_eq!(
            model.api_endpoint.unwrap(),
            Some("https://example.com".to_string())
        );
        assert_eq!(model.api_key.unwrap(), Some("secret".to_string()));
        assert_eq!(model.description.unwrap(), Some("A project".to_string()));
    }

    #[test]
    fn to_project_no_name_is_err() {
        let v = json!({ "golbat": false });
        assert!(v.to_project().is_err());
    }

    #[test]
    fn to_project_not_object_is_err() {
        let v = json!(42);
        assert!(v.to_project().is_err());
    }

    // ── JsonToModel — to_property ───────────────────────────────────────────────

    #[test]
    fn to_property_ok() {
        let v = json!({ "name": "color", "category": "string", "default_value": "red" });
        let model = v.to_property().unwrap();
        assert_eq!(model.name.unwrap(), "color");
        assert_eq!(model.default_value.unwrap(), Some("red".to_string()));
    }

    #[test]
    fn to_property_null_default_value_becomes_none() {
        // "null" string → stored as None (the "null" guard).
        let v = json!({ "name": "p", "category": "string", "default_value": "null" });
        let model = v.to_property().unwrap();
        assert_eq!(model.default_value.unwrap(), None);
    }

    #[test]
    fn to_property_no_name_is_err() {
        let v = json!({ "category": "string" });
        assert!(v.to_property().is_err());
    }

    #[test]
    fn to_property_no_category_is_err() {
        let v = json!({ "name": "p" });
        assert!(v.to_property().is_err());
    }

    #[test]
    fn to_property_not_object_is_err() {
        let v = json!(null);
        assert!(v.to_property().is_err());
    }

    // ── JsonToModel — to_route ──────────────────────────────────────────────────

    fn valid_multipoint() -> serde_json::Value {
        json!({ "type": "MultiPoint", "coordinates": [[1.0, 2.0], [3.0, 4.0]] })
    }

    #[test]
    fn to_route_ok() {
        let v = json!({ "name": "R1", "geofence_id": 7, "geometry": valid_multipoint() });
        let model = v.to_route().unwrap();
        assert_eq!(model.name.unwrap(), "R1");
        assert_eq!(model.geofence_id.unwrap(), 7u32);
    }

    #[test]
    fn to_route_no_name_is_err() {
        let v = json!({ "geofence_id": 1, "geometry": valid_multipoint() });
        assert!(v.to_route().is_err());
    }

    #[test]
    fn to_route_no_geofence_id_is_err() {
        let v = json!({ "name": "R1", "geometry": valid_multipoint() });
        assert!(v.to_route().is_err());
    }

    #[test]
    fn to_route_no_geometry_is_err() {
        let v = json!({ "name": "R1", "geofence_id": 1 });
        assert!(v.to_route().is_err());
    }

    #[test]
    fn to_route_invalid_geometry_is_err() {
        let v = json!({ "name": "R1", "geofence_id": 1, "geometry": "bad" });
        assert!(v.to_route().is_err());
    }

    #[test]
    fn to_route_not_object_is_err() {
        let v = json!(null);
        assert!(v.to_route().is_err());
    }

    // ── JsonToModel — to_tileserver ─────────────────────────────────────────────

    #[test]
    fn to_tileserver_ok() {
        let v = json!({ "name": "OSM", "url": "https://tile.openstreetmap.org/{z}/{x}/{y}.png" });
        let model = v.to_tileserver().unwrap();
        assert_eq!(model.name.unwrap(), "OSM");
        assert!(model.url.unwrap().contains("openstreetmap"));
    }

    #[test]
    fn to_tileserver_no_name_is_err() {
        let v = json!({ "url": "https://example.com" });
        assert!(v.to_tileserver().is_err());
    }

    #[test]
    fn to_tileserver_no_url_is_err() {
        let v = json!({ "name": "Tiles" });
        assert!(v.to_tileserver().is_err());
    }

    #[test]
    fn to_tileserver_not_object_is_err() {
        let v = json!([]);
        assert!(v.to_tileserver().is_err());
    }

    // ── JsonToModel — to_geofence_project ───────────────────────────────────────

    #[test]
    fn to_geofence_project_ok() {
        let v = json!({ "geofence_id": 3, "project_id": 7 });
        let model = v.to_geofence_project().unwrap();
        assert_eq!(model.geofence_id.unwrap(), 3u32);
        assert_eq!(model.project_id.unwrap(), 7u32);
    }

    #[test]
    fn to_geofence_project_missing_geofence_id_is_err() {
        let v = json!({ "project_id": 7 });
        assert!(v.to_geofence_project().is_err());
    }

    #[test]
    fn to_geofence_project_missing_project_id_is_err() {
        let v = json!({ "geofence_id": 3 });
        assert!(v.to_geofence_project().is_err());
    }

    #[test]
    fn to_geofence_project_not_object_is_err() {
        let v = json!(null);
        assert!(v.to_geofence_project().is_err());
    }

    // ── JsonToModel — to_geofence_property ──────────────────────────────────────

    #[test]
    fn to_geofence_property_with_explicit_geofence_id_arg() {
        let v = json!({ "property_id": 2, "value": "red" });
        let model = v.to_geofence_property(Some(9)).unwrap();
        assert_eq!(model.geofence_id.unwrap(), 9u32);
        assert_eq!(model.property_id.unwrap(), 2u32);
        assert_eq!(model.value.unwrap(), Some("red".to_string()));
    }

    #[test]
    fn to_geofence_property_geofence_id_from_json() {
        let v = json!({ "geofence_id": 5, "property_id": 3, "value": "blue" });
        let model = v.to_geofence_property(None).unwrap();
        assert_eq!(model.geofence_id.unwrap(), 5u32);
    }

    #[test]
    fn to_geofence_property_arg_overrides_json_geofence_id() {
        let v = json!({ "geofence_id": 99, "property_id": 3, "value": "x" });
        let model = v.to_geofence_property(Some(1)).unwrap();
        // Explicit arg wins.
        assert_eq!(model.geofence_id.unwrap(), 1u32);
    }

    #[test]
    fn to_geofence_property_empty_string_value_becomes_none() {
        let v = json!({ "property_id": 1, "value": "" });
        let model = v.to_geofence_property(Some(1)).unwrap();
        assert_eq!(model.value.unwrap(), None);
    }

    #[test]
    fn to_geofence_property_null_json_value_becomes_none() {
        let v = json!({ "property_id": 1, "value": null });
        let model = v.to_geofence_property(Some(1)).unwrap();
        assert_eq!(model.value.unwrap(), None);
    }

    #[test]
    fn to_geofence_property_numeric_value_stringified() {
        // Non-string, non-null value is stringified (e.g. JSON number).
        let v = json!({ "property_id": 1, "value": 42 });
        let model = v.to_geofence_property(Some(1)).unwrap();
        assert_eq!(model.value.unwrap(), Some("42".to_string()));
    }

    #[test]
    fn to_geofence_property_missing_geofence_id_is_err() {
        let v = json!({ "property_id": 2, "value": "x" });
        assert!(v.to_geofence_property(None).is_err());
    }

    #[test]
    fn to_geofence_property_missing_property_id_is_err() {
        let v = json!({ "geofence_id": 1 });
        assert!(v.to_geofence_property(None).is_err());
    }

    #[test]
    fn to_geofence_property_not_object_is_err() {
        let v = json!("bad");
        assert!(v.to_geofence_property(None).is_err());
    }
}
