use geojson::Feature;
use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_orm::{
    ConnectionTrait, DbBackend, FromQueryResult, JsonValue, Statement,
};
use std::collections::HashMap;

use crate::m20230203_214735_property_table::Property;
use crate::m20230203_231010_geofence_property_table::GeofenceProperty;

struct FeatProperties {
    id: u32,
    properties: Vec<(String, Option<Value>)>,
}

#[derive(Debug, FromQueryResult)]
struct PropertyType {
    id: u32,
    name: String,
}

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        log::info!(
            "[MIGRATION_12] Performing a one time insert to extract your geofence properties"
        );
        let db = manager.get_connection();
        let geofences = JsonValue::find_by_statement(Statement::from_sql_and_values(
            DbBackend::MySql,
            r#"SELECT * FROM geofence"#,
            [],
        ))
        .all(db)
        .await?;

        let mut properties: HashMap<String, String> = HashMap::new();
        let mut geofence_properties: Vec<FeatProperties> = vec![];

        for fence in geofences.into_iter() {
            let mut feat_properties = FeatProperties {
                id: match fence.get("id").unwrap().as_u64() {
                    Some(id) => id as u32,
                    None => continue,
                },
                properties: vec![],
            };
            if let Some(fence) = fence.as_object() {
                if let Some(area) = fence.get("area") {
                    let feature = Feature::from_json_value(area.clone());
                    if let Ok(feature) = feature {
                        for (key, value) in feature.properties_iter() {
                            let mut actual_value: Option<Value> = Some(value.clone().into());

                            if fence.contains_key(key) {
                                properties.insert(key.to_string(), "database".to_string());
                                actual_value = None
                            } else if let Some(val) = value.as_bool() {
                                properties.insert(key.to_string(), "boolean".to_string());
                                actual_value = Some(val.into());
                            } else if let Some(val) = value.as_f64() {
                                properties.insert(key.to_string(), "number".to_string());
                                actual_value = Some(val.into());
                            } else if let Some(_) = value.as_array() {
                                properties.insert(key.to_string(), "array".to_string());
                            } else if let Some(_) = value.as_object() {
                                properties.insert(key.to_string(), "object".to_string());
                            } else if let Some(value) = value.as_str() {
                                match value.parse::<f64>() {
                                    Ok(val) => {
                                        properties.insert(key.to_string(), "number".to_string());
                                        actual_value = Some(val.into());
                                    }
                                    Err(_) => {
                                        if value == "true" {
                                            properties
                                                .insert(key.to_string(), "boolean".to_string());
                                            actual_value = Some(true.into());
                                        } else if value == "false" {
                                            properties
                                                .insert(key.to_string(), "boolean".to_string());
                                            actual_value = Some(false.into());
                                        } else if value.starts_with("#") || value.starts_with("rgb")
                                        {
                                            properties.insert(key.to_string(), "color".to_string());
                                            actual_value = Some(value.to_string().into());
                                        } else if value.starts_with("{") {
                                            properties
                                                .insert(key.to_string(), "object".to_string());
                                            actual_value =
                                                match serde_json::from_str::<JsonValue>(value) {
                                                    Ok(val) => Some(val.into()),
                                                    Err(_) => None,
                                                };
                                        } else if value.starts_with("[") {
                                            properties.insert(key.to_string(), "array".to_string());
                                            actual_value =
                                                match serde_json::from_str::<JsonValue>(value) {
                                                    Ok(val) => Some(val.into()),
                                                    Err(_) => None,
                                                };
                                        } else {
                                            properties
                                                .insert(key.to_string(), "string".to_string());
                                            actual_value = Some(value.to_string().into());
                                        }
                                    }
                                };
                            }
                            feat_properties
                                .properties
                                .push((key.to_string(), actual_value));
                        }
                    }
                }
            }
            geofence_properties.push(feat_properties);
        }

        for (name, category) in properties.into_iter() {
            let default_value = match category.as_str() {
                "array" => Some("[]".to_string()),
                "object" => Some("{}".to_string()),
                _ => None,
            };
            let insert = Query::insert()
                .into_table(Property::Table)
                .columns([Property::Name, Property::Category, Property::DefaultValue])
                .values_panic([name.into(), category.into(), default_value.into()])
                .to_owned();
            manager.exec_stmt(insert).await?;
        }

        let properties = PropertyType::find_by_statement(Statement::from_sql_and_values(
            DbBackend::MySql,
            r#"SELECT * FROM property"#,
            [],
        ))
        .all(db)
        .await?
        .into_iter()
        .map(|p| (p.name, p.id))
        .collect::<HashMap<String, u32>>();

        for fence in geofence_properties.into_iter() {
            for (name, value) in fence.properties.into_iter() {
                if let Some(property_id) = properties.get(&name) {
                    if let Some(value) = value {
                        let insert = Query::insert()
                            .into_table(GeofenceProperty::Table)
                            .columns([
                                GeofenceProperty::GeofenceId,
                                GeofenceProperty::PropertyId,
                                GeofenceProperty::Value,
                            ])
                            .values_panic([
                                fence.id.into(),
                                property_id.clone().into(),
                                value.into(),
                            ])
                            .to_owned();
                        manager.exec_stmt(insert).await?;
                    } else {
                        let insert = Query::insert()
                            .into_table(GeofenceProperty::Table)
                            .columns([GeofenceProperty::GeofenceId, GeofenceProperty::PropertyId])
                            .values_panic([fence.id.into(), property_id.clone().into()])
                            .to_owned();
                        manager.exec_stmt(insert).await?;
                    }
                }
            }
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        log::info!("[MIGRATION_12] Deleting properties as part of the rollback process");
        match manager
            .get_connection()
            .execute(Statement::from_string(
                manager.get_database_backend(),
                r#"DELETE FROM property"#.to_owned(),
            ))
            .await
        {
            Ok(_) => Ok(()),
            Err(e) => Err(e),
        }
    }
}
