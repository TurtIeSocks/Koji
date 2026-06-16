//! Read paths for the geofence entity (single/all fetches, the recursive
//! descendants walk, and `search`). Split out of the former `geofence.rs`
//! god-file as a **pure relocation** — `impl super::Query`, no logic change.

use koji_core::KojiGeometryCollection;

use crate::error::ModelError;

use super::*;

use serde_json::json;

impl Query {
    /// Recursive descendants walk (Phase 2 §7): from `anchor` (a single root by
    /// id/name, or the whole forest = `parent IS NULL`), return a flat
    /// [`KojiGeometryCollection`] of geofences per `spec` — either the cumulative
    /// subtree ([`HierarchySpec::Depth`]) or exactly one level
    /// ([`HierarchySpec::Level`]). Each item's [`KojiMeta::ancestors`] is the
    /// ordered root→parent name path (excluding its own name) and `parent_id`
    /// comes from the row. Uses a bound-param recursive CTE
    /// ([`build_descendants_sql`]); no user input is interpolated.
    ///
    /// [`KojiMeta::ancestors`]: koji_core::KojiMeta::ancestors
    #[allow(clippy::result_large_err)]
    pub async fn descendants(
        db: &DatabaseConnection,
        anchor: Anchor,
        spec: HierarchySpec,
    ) -> Result<KojiGeometryCollection, ModelError> {
        let (sql, binds) = build_descendants_sql(&anchor, &spec);
        let rows = Entity::find()
            .from_raw_sql(Statement::from_sql_and_values(
                DbBackend::MySql,
                &sql,
                binds,
            ))
            .into_model::<DescendantRow>()
            .all(db)
            .await?;

        rows.into_iter()
            .map(|row| {
                let model = Model {
                    id: row.id,
                    name: row.name,
                    parent: row.parent,
                    created_at: row.created_at,
                    updated_at: row.updated_at,
                    mode: row.mode,
                    geometry: row.geometry,
                    geo_type: row.geo_type,
                    dragonite_area_id: row.dragonite_area_id,
                };
                let mut kg = model.to_koji_geometry()?;
                kg.meta.ancestors = ancestors_from_ancestry(&row.ancestry);
                Ok(kg)
            })
            .collect::<Result<KojiGeometryCollection, ModelError>>()
    }

    /// Returns a single Geofence model and it's related projects as tuple
    pub async fn get_one(db: &DatabaseConnection, id: String) -> Result<Model, ModelError> {
        let record = match id.parse::<u32>() {
            Ok(id) => Entity::find_by_id(id).one(db).await?,
            Err(_) => Entity::find().filter(Column::Name.eq(id)).one(db).await?,
        };
        if let Some(record) = record {
            Ok(record)
        } else {
            Err(ModelError::Geofence("Does not exist".to_string()))
        }
    }

    pub async fn get_one_json(db: &DatabaseConnection, id: String) -> Result<Json, ModelError> {
        match Query::get_one(db, id).await {
            Ok(record) => Ok(json!(record)),
            Err(err) => Err(err),
        }
    }

    pub async fn get_one_json_with_related(
        db: &DatabaseConnection,
        id: String,
    ) -> Result<Json, ModelError> {
        match Query::get_one(db, id).await {
            Ok(record) => {
                let mut json = json!(record);
                let json = json.as_object_mut().unwrap();
                if json.contains_key("area") {
                    json.remove("area");
                }
                json.insert(
                    "projects".to_string(),
                    json!(
                        record
                            .get_related_projects()
                            .into_model::<NameId>()
                            .all(db)
                            .await?
                            .into_iter()
                            .map(|p| p.id)
                            .collect::<Vec<u32>>()
                    ),
                );
                json.insert(
                    "routes".to_string(),
                    json!(record.get_related_routes().into_json().all(db).await?),
                );
                json.insert(
                    "properties".to_string(),
                    json!(
                        record
                            .get_related_properties()
                            .into_model::<FullPropertyModel>()
                            .all(db)
                            .await?
                            .into_iter()
                            .map(|prop| {
                                let property_id = prop.property_id;
                                let mut new_json = json!(prop.parse_db_value(&record));
                                new_json["property_id"] = property_id.into();
                                new_json
                            })
                            .collect::<Vec<Json>>()
                    ),
                );
                Ok(json!(json))
            }
            Err(err) => Err(err),
        }
    }

    /// Returns all Geofence models in the db
    pub async fn get_all(db: &DatabaseConnection) -> Result<Vec<Model>, DbErr> {
        Entity::find().all(db).await
    }

    /// Returns all Geofence models in the db
    pub async fn get_all_json(db: &DatabaseConnection) -> Result<Vec<Json>, DbErr> {
        Ok(Query::get_all(db).await?.to_json())
    }

    /// Fetch all geofence rows and map each `Model` to a `KojiGeometry`,
    /// collecting into a `KojiGeometryCollection`. Returns `ModelError` to match
    /// `to_koji_geometry` and the sibling `get_one` methods. The property/name
    /// helper maps are not needed here — `to_koji_geometry` reads only the model's
    /// own columns.
    #[allow(clippy::result_large_err)]
    pub async fn get_all_koji(
        db: &DatabaseConnection,
    ) -> Result<koji_core::KojiGeometryCollection, ModelError> {
        let results = Query::get_all(db).await?;
        results
            .iter()
            .map(|result| result.to_koji_geometry())
            .collect::<Result<koji_core::KojiGeometryCollection, ModelError>>()
    }

    /// Fetch one geofence row by id and map it via `to_koji_geometry`. Returns
    /// `ModelError` to match the sibling Koji read methods.
    #[allow(clippy::result_large_err)]
    pub async fn get_one_koji(
        db: &DatabaseConnection,
        id: String,
    ) -> Result<koji_core::KojiGeometry, ModelError> {
        let result = Query::get_one(db, id).await?;
        result.to_koji_geometry()
    }

    /// Returns all Geofence models in the db without their features
    pub async fn get_all_no_fences(
        db: &DatabaseConnection,
    ) -> Result<Vec<GeofenceNoGeometry>, DbErr> {
        Entity::find()
            .select_only()
            .column(Column::Id)
            .column(Column::Name)
            .column(Column::Mode)
            .column(Column::GeoType)
            .column(Column::Parent)
            .order_by(Column::Name, Order::Asc)
            .into_model::<GeofenceNoGeometry>()
            .all(db)
            .await
    }

    pub async fn search(db: &DatabaseConnection, search: String) -> Result<Vec<Json>, DbErr> {
        Entity::find()
            .filter(Column::Name.like(format!("%{}%", search).as_str()))
            .into_json()
            .all(db)
            .await
    }
}
