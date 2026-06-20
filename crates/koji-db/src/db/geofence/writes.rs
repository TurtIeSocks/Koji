//! Write paths for the geofence entity (upserts, deletes, related
//! property/project/route mutations, `KojiGeometry`-driven persistence, and the
//! column `assign`). Split out of the former `geofence.rs` god-file as a **pure
//! relocation** — `impl super::Query`, no logic change.

use std::collections::HashMap;

use koji_core::{KojiGeometry, KojiGeometryCollection, UnknownId};

use crate::error::ModelError;

use super::*;

use serde_json::json;

impl Query {
    pub async fn update_related_route_names<C: ConnectionTrait>(
        conn: &C,
        old_model: &Model,
        new_name: String,
    ) -> Result<UpdateResult, DbErr> {
        route::Entity::update_many()
            .col_expr(route::Column::Name, Expr::value(new_name))
            .filter(route::Column::GeofenceId.eq(old_model.id.to_owned()))
            .filter(route::Column::Name.eq(old_model.name.to_owned()))
            .exec(conn)
            .await
    }

    pub async fn upsert_related_properties<C: ConnectionTrait>(
        db: &C,
        json: &serde_json::Value,
        geofence_id: u32,
    ) -> Result<(), ModelError> {
        if let Some(properties) = json.get("properties")
            && let Some(properties) = properties.as_array()
        {
            let mut existing = vec![];
            let mut new_props = vec![];
            properties.iter().for_each(|property| {
                if let Some(prop_map) = property.as_object() {
                    if prop_map.contains_key("property_id") {
                        existing.push(property.clone())
                    } else {
                        new_props.push(property.clone())
                    }
                }
            });

            // Sequential (not try_join_all): may run inside a DatabaseTransaction.
            let mut upserted_props = Vec::with_capacity(new_props.len());
            for result in new_props.clone() {
                upserted_props.push(property::Query::upsert(db, 0, result).await?);
            }

            upserted_props
                .into_iter()
                .enumerate()
                .for_each(|(i, prop)| {
                    existing.push(json!({
                        "value": new_props[i]["value"],
                        "property_id": prop.id,
                        "geofence_id": geofence_id,
                    }))
                });

            geofence_property::Query::update_properties_by_geofence(
                db,
                &existing,
                Some(geofence_id),
            )
            .await?;
        };
        Ok(())
    }

    pub async fn upsert_related_projects<C: ConnectionTrait>(
        db: &C,
        json: &serde_json::Value,
        geofence_id: u32,
    ) -> Result<(), DbErr> {
        if let Some(projects) = json.get("projects")
            && let Some(projects) = projects.as_array()
        {
            geofence_project::Query::upsert_related_by_geofence_id(db, projects, geofence_id)
                .await?;
        };
        Ok(())
    }

    /// Updates or creates a Geofence model, returns a model struct
    pub async fn upsert<C: ConnectionTrait>(db: &C, id: u32, json: Json) -> Result<Model, ModelError> {
        let mut json = json;

        let mut new_model = json.to_geofence()?;

        let old_model = if id == 0 {
            Query::get_one(db, new_model.name.clone().unwrap())
                .await
                .ok()
        } else {
            Entity::find_by_id(id).one(db).await?
        };

        let name = new_model.name.as_ref();

        let model = if let Some(old_model) = old_model {
            if old_model.name.ne(name) {
                Query::update_related_route_names(db, &old_model, name.clone()).await?;
            };
            new_model.id = Set(old_model.id);
            new_model.update(db).await?
        } else {
            let model = new_model.insert(db).await?;
            let prop_name_model =
                geofence_property::Query::add_db_property(db, model.id, "name").await?;
            if let Some(properties) = json["properties"].as_array_mut() {
                properties.push(json!({
                    "property_id": prop_name_model.property_id,
                    "geofence_id": prop_name_model.geofence_id,
                }))
            }
            model
        };
        Query::upsert_related_projects(db, &json, model.id).await?;
        Query::upsert_related_properties(db, &json, model.id).await?;
        Ok(model)
    }

    /// Updates or creates a Geofence model, returns a json
    pub async fn upsert_json_return<C: ConnectionTrait>(
        db: &C,
        id: u32,
        json: Json,
    ) -> Result<Json, ModelError> {
        let result = Query::upsert(db, id, json).await?;
        Ok(result.to_json())
    }

    /// Deletes a Geofence model from db
    pub async fn delete(db: &DatabaseConnection, id: u32) -> Result<DeleteResult, DbErr> {
        let record = Entity::delete_by_id(id).exec(db).await?;
        Ok(record)
    }

    /// Persist one geofence directly from a `KojiGeometry` (no geojson `Feature`
    /// round-trip). Reads name/mode/projects/parent/id from the legacy
    /// `__`-prefixed `meta.extra` passthrough (the admin-panel internal
    /// round-trip), falling back to the plain key and then the typed `KojiMeta`
    /// field. Any remaining non-`__` `extra` entry becomes a custom property —
    /// byte-identical to the deleted `Feature`-based path, which filtered on the
    /// `__` prefix. Delegates to the property-aware [`Query::upsert`] so parent
    /// association, projects, and property categories are unchanged.
    async fn upsert_koji_item(
        conn: &DatabaseConnection,
        item: &KojiGeometry,
        parent_map: &mut HashMap<String, UnknownId>,
    ) -> Result<Model, ModelError> {
        let GeofenceUpsertInputs {
            id,
            name,
            new_map,
            parent,
        } = build_geofence_upsert_map(item)?;
        if let Some(parent) = parent {
            parent_map.insert(name, parent);
        }
        Query::upsert(conn, id, new_map).await
    }

    pub async fn upsert_from_geometry(
        conn: &DatabaseConnection,
        area: &KojiGeometryCollection,
    ) -> Result<(), ModelError> {
        let mut parent_map = HashMap::<String, UnknownId>::new();
        for item in &area.items {
            // Persist directly from the Koji item — name/mode/parent/projects
            // come from `KojiMeta` (with `__`-prefixed `extra` passthrough for
            // the admin internal round-trip); no geojson `Feature` round-trip.
            Query::upsert_koji_item(conn, item, &mut parent_map).await?;
        }
        if !parent_map.is_empty() {
            // ensures it exists before running the try_join_all
            property::Query::get_or_create_db_prop(conn, "parent").await?;
            future::try_join_all(
                parent_map
                    .into_iter()
                    .map(|(name, parent)| Query::associate_parent(conn, name, parent)),
            )
            .await?;
        }

        Ok(())
    }

    async fn associate_parent(
        db: &DatabaseConnection,
        name: String,
        parent: UnknownId,
    ) -> Result<(), ModelError> {
        let model = Query::get_one(db, name).await?;
        let parent_model = Query::get_one(db, parent.to_string()).await?;
        let mut new_model: ActiveModel = model.into();
        new_model.parent = Set(Some(parent_model.id));
        let model = new_model.update(db).await?;
        geofence_property::Query::add_db_property(db, model.id, "parent").await?;
        Ok(())
    }

    pub async fn assign(
        db: &DatabaseConnection,
        id: u32,
        property: String,
        payload: serde_json::Value,
    ) -> Result<Model, ModelError> {
        let column = Column::from_str(&property);

        if let Ok(column) = column {
            let model = Entity::find_by_id(id).one(db).await?;
            if let Some(model) = model {
                let mut model: ActiveModel = model.into();
                if let Column::Parent = column {
                    match payload.as_u64() {
                        Some(id) => {
                            if id == 0 {
                                model.parent = Set(None);
                            } else {
                                model.parent = Set(Some(id as u32));
                            }
                        }
                        None => {
                            return Err(ModelError::Geofence(
                                "No valid parent_id found".to_string(),
                            ));
                        }
                    }
                }
                let model = model.update(db).await?;
                Ok(model)
            } else {
                Err(ModelError::Geofence("Model not found".to_string()))
            }
        } else {
            Err(ModelError::Geofence("Invalid property".to_string()))
        }
    }
}
