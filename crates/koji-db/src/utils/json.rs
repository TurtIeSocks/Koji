use koji_core::Precision;
use std::{collections::HashMap, str::FromStr};

use geojson::GeoJson;
use sea_orm::Set;
use serde_json::Value;

use crate::{
    WebhookMethod, WebhookMode,
    db::{
        geofence, geofence_property, project, property, route, sea_orm_active_enums::Category,
        tile_server, webhook,
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
    fn to_property(&self) -> Result<property::ActiveModel, ModelError>;
    fn to_route(&self) -> Result<route::ActiveModel, ModelError>;
    fn to_tileserver(&self) -> Result<tile_server::ActiveModel, ModelError>;
    fn to_webhook(&self) -> Result<webhook::ActiveModel, ModelError>;
}

/// The JSON object behind `v`, or the module's standard "not an object" error
/// wrapped in the caller's entity variant.
#[allow(clippy::result_large_err)]
fn as_obj(
    v: &Value,
    err: impl FnOnce(String) -> ModelError,
) -> Result<&serde_json::Map<String, Value>, ModelError> {
    v.as_object()
        .ok_or_else(|| err(format!("model is not an object: {v:?}")))
}

/// A required string property, or the standard "does not have X" error.
#[allow(clippy::result_large_err)]
fn req_str<'a>(
    v: &'a Value,
    key: &str,
    err: impl FnOnce(String) -> ModelError,
) -> Result<&'a str, ModelError> {
    v.get(key)
        .and_then(|x| x.as_str())
        .ok_or_else(|| err(format!("model does not have a {key} property: {v:?}")))
}

/// Validate the `geometry` member as geojson and re-serialize it to the stored
/// `GeoJson::Geometry` wrapper form.
#[allow(clippy::result_large_err)]
fn parse_geometry(v: &Value, err: impl Fn(String) -> ModelError) -> Result<Value, ModelError> {
    let geometry = v
        .get("geometry")
        .ok_or_else(|| err(format!("model does not have a geometry object: {v:?}")))?;
    let geometry = serde_json::from_value::<geojson::Geometry>(geometry.to_owned())
        .map_err(|e| err(format!("geometry is invalid: {e:?}")))?;
    Ok(serde_json::to_value(GeoJson::Geometry(geometry)).expect("geojson serializes"))
}

impl JsonToModel for Value {
    fn to_geofence(&self) -> Result<geofence::ActiveModel, ModelError> {
        let incoming = as_obj(self, ModelError::Geofence)?;
        let name = req_str(self, "name", ModelError::Geofence)?;
        let value = parse_geometry(self, ModelError::Geofence)?;
        let mode = get_enum(
            incoming
                .get("mode")
                .map(|mode| mode.as_str().unwrap_or("unset").to_string()),
        );
        let parent = incoming
            .get("parent")
            .and_then(|v| v.as_u64())
            .map(|parent| parent as u32);
        Ok(geofence::ActiveModel {
            name: Set(name.to_string()),
            geometry: Set(value),
            parent: Set(parent),
            mode: Set(mode),
            ..Default::default()
        })
    }

    fn to_geofence_property(
        &self,
        geofence_id: Option<u32>,
    ) -> Result<geofence_property::ActiveModel, ModelError> {
        let object = as_obj(self, |_| {
            ModelError::GeofenceProperty(format!("invalid object {self:?}"))
        })?;
        let geofence_id = match geofence_id {
            Some(geofence_id) => Some(geofence_id as u64),
            None => self.get("geofence_id").and_then(|v| v.as_u64()),
        };
        let Some(geofence_id) = geofence_id else {
            return Err(ModelError::GeofenceProperty(format!(
                "geofence_id not found: {object:?}"
            )));
        };
        let Some(property_id) = self.get("property_id").and_then(|v| v.as_u64()) else {
            return Err(ModelError::GeofenceProperty(format!(
                "property_id not found: {object:?}"
            )));
        };
        let value = match self.get("value") {
            Some(value) => match value.as_str() {
                Some(s) if !s.is_empty() => Some(s.to_string()),
                Some(_) => None,
                None if value == &Value::Null => None,
                None => Some(value.to_string()),
            },
            None => None,
        };
        Ok(geofence_property::ActiveModel {
            property_id: Set(property_id as u32),
            geofence_id: Set(geofence_id as u32),
            value: Set(value),
            ..Default::default()
        })
    }

    fn to_project(&self) -> Result<project::ActiveModel, ModelError> {
        let incoming = as_obj(self, ModelError::Project)?;
        let name = req_str(self, "name", ModelError::Project)?;
        let description = incoming
            .get("description")
            .and_then(|v| v.as_str())
            .map(|description| description.to_string());
        Ok(project::ActiveModel {
            name: Set(name.to_string()),
            description: Set(description),
            ..Default::default()
        })
    }

    fn to_property(&self) -> Result<property::ActiveModel, ModelError> {
        let incoming = as_obj(self, ModelError::Property)?;
        let name = req_str(self, "name", ModelError::Property)?;
        let category = incoming
            .get("category")
            .and_then(|v| v.as_str())
            .map(|category| get_category_enum(category.to_string()))
            .ok_or_else(|| {
                ModelError::Property(format!("model does not have a category property: {self:?}"))
            })?;
        let default_value = incoming
            .get("default_value")
            .map(|default_value| match default_value.as_str() {
                Some(s) => s.to_string(),
                None => default_value.to_string(),
            })
            .filter(|v| v != "null");
        Ok(property::ActiveModel {
            name: Set(name.to_string()),
            category: Set(category),
            default_value: Set(default_value),
            ..Default::default()
        })
    }

    fn to_route(&self) -> Result<route::ActiveModel, ModelError> {
        let incoming = as_obj(self, ModelError::Route)?;
        let name = req_str(self, "name", ModelError::Route)?;
        let Some(geofence_id) = incoming.get("geofence_id").and_then(|v| v.as_u64()) else {
            return Err(ModelError::Route(format!(
                "model does not have a geofence_id property: {self:?}"
            )));
        };
        let value = parse_geometry(self, ModelError::Route)?;
        let mode = get_enum(
            incoming
                .get("mode")
                .map(|mode| mode.as_str().unwrap_or("unset").to_string()),
        );
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

    fn to_webhook(&self) -> Result<webhook::ActiveModel, ModelError> {
        let incoming = as_obj(self, ModelError::Custom)?;
        let name = req_str(self, "name", ModelError::Custom)?;
        let url = req_str(self, "url", ModelError::Custom)?;
        let secret = incoming
            .get("secret")
            .and_then(|v| v.as_str())
            .map(|secret| secret.to_string());
        // Absent OR explicit JSON `null` both mean "no topics" (empty array =
        // fires on all events). The distinction matters because
        // `koji_resource!`'s generated `CreateWebhook` DTO has no
        // `skip_serializing_if` (only its `Patch…` twin does), so an omitted
        // `topics` in a POST body round-trips through `Option<Value>::None`
        // back out as an explicit `"topics": null` — mirrors `headers`'
        // None-or-Null handling below.
        let topics = match incoming.get("topics") {
            None | Some(Value::Null) => Value::Array(vec![]),
            Some(value) => value.clone(),
        };
        let active = incoming
            .get("active")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        let project_id = incoming
            .get("project_id")
            .and_then(|v| v.as_u64())
            .map(|project_id| project_id as u32);
        let mode = match incoming.get("mode") {
            Some(mode) => serde_json::from_value::<WebhookMode>(mode.clone())
                .map_err(|err| ModelError::Custom(format!("mode is invalid: {err:?}")))?,
            None => WebhookMode::Event,
        };
        let method = match incoming.get("method") {
            Some(method) => serde_json::from_value::<WebhookMethod>(method.clone())
                .map_err(|err| ModelError::Custom(format!("method is invalid: {err:?}")))?,
            None => WebhookMethod::Get,
        };
        let headers = match incoming.get("headers") {
            None | Some(Value::Null) => None,
            Some(value) if value.is_object() => Some(value.clone()),
            Some(value) => {
                return Err(ModelError::Custom(format!(
                    "headers must be an object: {value:?}"
                )));
            }
        };
        Ok(webhook::ActiveModel {
            name: Set(name.to_string()),
            url: Set(url.to_string()),
            secret: Set(secret),
            topics: Set(topics),
            active: Set(active),
            project_id: Set(project_id),
            mode: Set(mode),
            method: Set(method),
            headers: Set(headers),
            ..Default::default()
        })
    }

    fn to_tileserver(&self) -> Result<tile_server::ActiveModel, ModelError> {
        as_obj(self, ModelError::TileServer)?;
        let name = req_str(self, "name", ModelError::TileServer)?;
        let url = req_str(self, "url", ModelError::TileServer)?;
        Ok(tile_server::ActiveModel {
            name: Set(name.to_string()),
            url: Set(url.to_string()),
            ..Default::default()
        })
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
    }

    #[test]
    fn to_project_ok_full() {
        let v = json!({
            "name": "Full",
            "description": "A project"
        });
        let model = v.to_project().unwrap();
        assert_eq!(model.name.unwrap(), "Full");
        assert_eq!(model.description.unwrap(), Some("A project".to_string()));
    }

    #[test]
    fn to_project_no_name_is_err() {
        let v = json!({ "description": "no name" });
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

    // ── JsonToModel — to_webhook ────────────────────────────────────────────────

    #[test]
    fn to_webhook_minimal_defaults() {
        let m = json!({"name": "n", "url": "http://x"})
            .to_webhook()
            .unwrap();
        assert_eq!(m.name.unwrap(), "n");
        assert_eq!(m.url.unwrap(), "http://x");
        assert_eq!(m.topics.unwrap(), json!([]));
        assert!(m.active.unwrap());
        assert_eq!(m.mode.unwrap(), crate::WebhookMode::Event);
        assert_eq!(m.method.unwrap(), crate::WebhookMethod::Get);
        assert_eq!(m.secret.unwrap(), None);
        assert_eq!(m.project_id.unwrap(), None);
        assert_eq!(m.headers.unwrap(), None);
    }

    #[test]
    fn to_webhook_requires_name_and_url() {
        assert!(json!({"url": "http://x"}).to_webhook().is_err());
        assert!(json!({"name": "n"}).to_webhook().is_err());
    }

    #[test]
    fn to_webhook_full_row() {
        let m = json!({
            "name": "reactmap", "url": "http://rm/reload", "secret": "s",
            "topics": ["geofence.updated"], "active": false, "project_id": 7,
            "mode": "ping", "method": "POST", "headers": {"react-map-secret": "v"}
        })
        .to_webhook()
        .unwrap();
        assert_eq!(m.name.unwrap(), "reactmap");
        assert_eq!(m.url.unwrap(), "http://rm/reload");
        assert_eq!(m.secret.unwrap(), Some("s".to_string()));
        assert_eq!(m.topics.unwrap(), json!(["geofence.updated"]));
        assert!(!m.active.unwrap());
        assert_eq!(m.project_id.unwrap(), Some(7));
        assert_eq!(m.mode.unwrap(), crate::WebhookMode::Ping);
        assert_eq!(m.method.unwrap(), crate::WebhookMethod::Post);
        assert_eq!(m.headers.unwrap(), Some(json!({"react-map-secret": "v"})));
    }

    #[test]
    fn to_webhook_non_object_headers_is_err() {
        let v = json!({"name": "n", "url": "http://x", "headers": "not-an-object"});
        assert!(v.to_webhook().is_err());
    }

    #[test]
    fn to_webhook_not_object_is_err() {
        assert!(json!(42).to_webhook().is_err());
    }

    #[test]
    fn to_webhook_explicit_null_topics_defaults_to_empty_array() {
        // Explicit JSON `null` (not just an absent key) must default the same
        // way an absent key does. Reachable via the API: `koji_resource!`'s
        // generated `CreateWebhook` DTO has no `skip_serializing_if`, so an
        // omitted `topics` in a POST body round-trips `Option<Value>::None`
        // back out as `"topics": null` — mirrors `headers`' None-or-Null arm.
        let m = json!({"name": "n", "url": "http://x", "topics": null})
            .to_webhook()
            .unwrap();
        assert_eq!(m.topics.unwrap(), json!([]));
    }

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
