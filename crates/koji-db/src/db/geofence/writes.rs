//! Write paths for the geofence entity (upserts, deletes, related
//! property/project/route mutations, and the column `assign`). Split out of the
//! former `geofence.rs` god-file as a **pure relocation** — `impl super::Query`,
//! no logic change.

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
    pub async fn upsert<C: ConnectionTrait>(
        db: &C,
        id: u32,
        json: Json,
    ) -> Result<Model, ModelError> {
        let mut json = json;

        let mut new_model = json.to_geofence()?;

        // Keep the persisted bbox columns (migration
        // `m20260714_000001_geofence_bbox_columns`) fresh on every create/update —
        // `to_geofence` always sets `geometry`, so `try_as_ref` is never `None`
        // here, but the `if let` avoids a panic if that ever changes.
        if let Some(geometry) = new_model.geometry.try_as_ref() {
            let bbox = geometry_bbox_lnglat(geometry);
            let [min_lng, min_lat, max_lng, max_lat] = bbox
                .map(|[min_lng, min_lat, max_lng, max_lat]| {
                    [Some(min_lng), Some(min_lat), Some(max_lng), Some(max_lat)]
                })
                .unwrap_or([None, None, None, None]);
            new_model.min_lng = Set(min_lng);
            new_model.min_lat = Set(min_lat);
            new_model.max_lng = Set(max_lng);
            new_model.max_lat = Set(max_lat);
        }

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

    /// Set (or clear, with `None`) a geofence's parent. Replaces the old
    /// stringly `assign(column_name, payload)`: every caller passed the literal
    /// "parent", and any OTHER valid column name silently no-opped (fetch,
    /// zero-field update, Ok).
    pub async fn set_parent<C: ConnectionTrait>(
        db: &C,
        id: u32,
        parent: Option<u32>,
    ) -> Result<Model, ModelError> {
        let model = Entity::find_by_id(id).one(db).await?;
        let Some(model) = model else {
            return Err(ModelError::Geofence("Model not found".to_string()));
        };
        let mut model: ActiveModel = model.into();
        model.parent = Set(parent);
        Ok(model.update(db).await?)
    }
}
