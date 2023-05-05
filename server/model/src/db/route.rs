//! SeaORM Entity. Generated by sea-orm-codegen 0.10.1

use super::*;

use geojson::{self, GeoJson, Geometry};
use sea_orm::{entity::prelude::*, FromQueryResult, Order, QueryOrder, QuerySelect};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::str::FromStr;

use crate::{
    api::{args::AdminReqParsed, GeoFormats, ToCollection, ToFeature},
    db::sea_orm_active_enums::Type,
    error::ModelError,
    utils::{get_enum, json::JsonToModel, parse_order},
};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "route")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: u32,
    pub geofence_id: u32,
    pub name: String,
    pub description: Option<String>,
    pub mode: Type,
    pub geometry: Json,
    #[sea_orm(column_type = "custom(\"MEDIUMINT UNSIGNED\")", nullable)]
    pub points: u32,
    pub created_at: DateTimeUtc,
    pub updated_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::geofence::Entity",
        from = "Column::GeofenceId",
        to = "super::geofence::Column::Id",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    Geofence,
}

impl Related<super::geofence::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Geofence.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

#[derive(Debug, Serialize, Deserialize, FromQueryResult)]
pub struct RouteNoGeometry {
    pub id: u32,
    pub geofence_id: u32,
    pub name: String,
    pub description: Option<String>,
    pub mode: Type,
    pub points: u32,
    pub created_at: DateTimeUtc,
    pub updated_at: DateTimeUtc,
}

#[derive(Serialize, Deserialize, FromQueryResult)]
pub struct OnlyGeofenceId {
    pub geofence_id: u32,
}

impl ToFeatureFromModel for Model {
    fn to_feature(self, internal: bool) -> Result<Feature, ModelError> {
        let Self {
            geometry,
            name,
            geofence_id,
            id,
            mode,
            ..
        } = self;

        let geometry = Geometry::from_json_value(geometry)?;
        let mut feature = geometry.to_feature(Some(Type::CirclePokemon));

        if internal {
            feature.id = Some(geojson::feature::Id::String(format!(
                "{}__{}__KOJI",
                id,
                mode.to_value(),
            )));
        }
        feature.set_property(if internal { "__name" } else { "name" }, name.clone());
        feature.set_property(if internal { "__id" } else { "id" }, id);
        feature.set_property(if internal { "__mode" } else { "mode" }, mode.to_value());
        feature.set_property(
            if internal {
                "__geofence_id"
            } else {
                "geofence_id"
            },
            geofence_id.clone(),
        );
        Ok(feature)
    }
}
pub struct Query;

impl Query {
    pub async fn paginate(
        db: &DatabaseConnection,
        args: AdminReqParsed,
    ) -> Result<PaginateResults<Vec<Json>>, DbErr> {
        let column = Column::from_str(&args.sort_by).unwrap_or(Column::Name);

        let mut paginator = Entity::find()
            .select_only()
            .column(Column::Id)
            .column(Column::Name)
            .column(Column::Mode)
            .column(Column::Points)
            .column(Column::GeofenceId)
            .column(Column::Description)
            .column(Column::CreatedAt)
            .column(Column::UpdatedAt)
            .order_by(column, parse_order(&args.order))
            .filter(Column::Name.like(format!("%{}%", args.q).as_str()));

        if let Some(geofence_id) = args.geofenceid {
            paginator = paginator.filter(Column::GeofenceId.eq(geofence_id));
        }
        if let Some(mode) = args.mode {
            paginator = paginator.filter(Column::Mode.eq(mode));
        }
        if let Some(pointsmin) = args.pointsmin {
            if let Some(pointsmax) = args.pointsmax {
                paginator = paginator.filter(Column::Points.between(pointsmin, pointsmax));
            } else {
                paginator = paginator.filter(Column::Points.gt(pointsmin));
            }
        } else if let Some(pointsmax) = args.pointsmax {
            paginator = paginator.filter(Column::Points.lt(pointsmax));
        }

        let paginator = paginator
            .into_model::<RouteNoGeometry>()
            .paginate(db, args.per_page);
        let total = paginator.num_items_and_pages().await?;

        let results: Vec<Json> = paginator
            .fetch_page(args.page)
            .await?
            .into_iter()
            .map(|model| {
                json!({
                    "id": model.id,
                    "geofence_id": model.geofence_id,
                    "name": model.name,
                    "description": model.description,
                    "points": model.points,
                    "mode": model.mode,
                })
            })
            .collect();

        Ok(PaginateResults {
            results,
            total: total.number_of_items,
            has_prev: total.number_of_pages == args.page + 1,
            has_next: args.page + 1 < total.number_of_pages,
        })
    }

    /// Returns all Geofence models in the db
    pub async fn get_all(db: &DatabaseConnection) -> Result<Vec<Model>, DbErr> {
        Entity::find().all(db).await
    }

    pub async fn get_json_cache(db: &DatabaseConnection) -> Result<Vec<sea_orm::JsonValue>, DbErr> {
        let results = Query::get_all_no_fences(db)
            .await?
            .into_iter()
            .map(|record| {
                let mut json = json!(record);
                json["geo_type"] = Json::String("MultiPoint".to_string());
                json
            })
            .collect();
        Ok(results)
    }

    /// Returns all Geofence models in the db without their features
    pub async fn get_all_no_fences(db: &DatabaseConnection) -> Result<Vec<RouteNoGeometry>, DbErr> {
        Entity::find()
            .select_only()
            .column(Column::Id)
            .column(Column::GeofenceId)
            .column(Column::Name)
            .column(Column::Description)
            .column(Column::Mode)
            .column(Column::Points)
            .column(Column::CreatedAt)
            .column(Column::UpdatedAt)
            .order_by(Column::Name, Order::Asc)
            .into_model::<RouteNoGeometry>()
            .all(db)
            .await
    }

    /// Creates a new Geofence model, only used from admin panel when creating a single geofence.
    /// Does not try to remove internal props since they do not exist yet
    pub async fn create(db: &DatabaseConnection, incoming: Model) -> Result<Model, DbErr> {
        let new_fence = Geometry::from_json_value(incoming.geometry);
        match new_fence {
            Ok(new_feature) => {
                let value = GeoJson::Geometry(new_feature).to_json_value();
                ActiveModel {
                    name: Set(incoming.name.to_owned()),
                    geofence_id: Set(incoming.geofence_id),
                    geometry: Set(value),
                    mode: Set(incoming.mode),
                    description: Set(incoming.description),
                    created_at: Set(Utc::now()),
                    updated_at: Set(Utc::now()),
                    ..Default::default()
                }
                .insert(db)
                .await
            }
            Err(err) => Err(DbErr::Custom(format!(
                "New area did not have valid geometry {:?}",
                err
            ))),
        }
    }

    pub async fn get_one(db: &DatabaseConnection, id: String) -> Result<Model, ModelError> {
        let record = match id.parse::<u32>() {
            Ok(id) => Entity::find_by_id(id).one(db).await?,
            Err(_) => Entity::find().filter(Column::Name.eq(id)).one(db).await?,
        };
        if let Some(record) = record {
            Ok(record)
        } else {
            Err(ModelError::Route("Does not exist".to_string()))
        }
    }

    pub async fn get_one_json(db: &DatabaseConnection, id: String) -> Result<Json, ModelError> {
        match Query::get_one(db, id).await {
            Ok(record) => Ok(json!(record)),
            Err(err) => Err(err),
        }
    }

    pub async fn get_one_feature(
        db: &DatabaseConnection,
        id: String,
        internal: bool,
    ) -> Result<Feature, ModelError> {
        match Query::get_one(db, id).await {
            Ok(record) => record.to_feature(internal),
            Err(err) => Err(err),
        }
    }

    pub async fn update(
        db: &DatabaseConnection,
        id: u32,
        new_model: Model,
    ) -> Result<Model, DbErr> {
        let old_model: Option<Model> = Entity::find_by_id(id).one(db).await?;
        let new_geometry = Geometry::from_json_value(new_model.geometry);
        if let Ok(new_geometry) = new_geometry {
            let value = GeoJson::Geometry(new_geometry).to_json_value();

            let mut old_model: ActiveModel = old_model.unwrap().into();
            old_model.name = Set(new_model.name.to_owned());
            old_model.description = Set(new_model.description.to_owned());
            old_model.geofence_id = Set(new_model.geofence_id);
            old_model.geometry = Set(value);
            old_model.mode = Set(new_model.mode);
            old_model.updated_at = Set(Utc::now());
            old_model.update(db).await
        } else {
            Err(DbErr::Custom("New geometry was not valid".to_string()))
        }
    }

    pub async fn delete(db: &DatabaseConnection, id: u32) -> Result<DeleteResult, DbErr> {
        let record = Entity::delete_by_id(id).exec(db).await?;
        Ok(record)
    }

    /// Returns a feature for a route queried by name
    pub async fn feature_from_name(
        conn: &DatabaseConnection,
        name: String,
        internal: bool,
    ) -> Result<Feature, ModelError> {
        let item = Entity::find()
            .filter(Column::Name.eq(Value::String(Some(Box::new(name.to_string())))))
            .one(conn)
            .await?;
        if let Some(item) = item {
            item.to_feature(internal)
        } else {
            Err(ModelError::Custom("Route not found".to_string()))
        }
    }

    /// Returns a feature for a route queried by name
    pub async fn feature(
        conn: &DatabaseConnection,
        id: u32,
        internal: bool,
    ) -> Result<Feature, ModelError> {
        let item = Entity::find_by_id(id).one(conn).await?;
        if let Some(item) = item {
            item.to_feature(internal)
        } else {
            Err(ModelError::Custom("Route not found".to_string()))
        }
    }

    /// Returns all route models as a FeatureCollection,
    pub async fn as_collection(
        conn: &DatabaseConnection,
        internal: bool,
    ) -> Result<FeatureCollection, DbErr> {
        let items = Entity::find()
            .order_by(Column::Name, Order::Asc)
            .all(conn)
            .await?;
        let items: Vec<Feature> = items
            .into_iter()
            .filter_map(|item| item.to_feature(internal).ok())
            .collect();

        Ok(items.to_collection(None, None))
    }

    pub async fn upsert(db: &DatabaseConnection, id: u32, json: Json) -> Result<Model, ModelError> {
        let old_model = Entity::find_by_id(id).one(db).await?;
        let mut new_model = json.to_route()?;

        let model = if let Some(old_model) = old_model {
            new_model.id = Set(old_model.id);
            new_model.update(db).await?
        } else {
            new_model.insert(db).await?
        };
        Ok(model)
    }

    pub async fn upsert_json_return(
        db: &DatabaseConnection,
        id: u32,
        json: Json,
    ) -> Result<Json, ModelError> {
        let result = Query::upsert(db, id, json).await?;
        Ok(json!(result))
    }

    async fn upsert_feature(
        conn: &DatabaseConnection,
        feat: Feature,
        existing: &HashMap<String, RouteNoGeometry>,
        inserts_updates: &mut InsertsUpdates<ActiveModel>,
    ) -> Result<(), DbErr> {
        if let Some(name) = feat.property("__name") {
            if let Some(name) = name.as_str() {
                if let Some(mode) = feat.property("__mode") {
                    let mode = if let Some(mode) = mode.as_str() {
                        Some(mode.to_string())
                    } else {
                        None
                    };
                    let mode = get_enum(mode);
                    let geofence_id = if let Some(fence_id) = feat.property("__geofence_id") {
                        fence_id.as_u64()
                    } else {
                        let geofence = geofence::Entity::find()
                            .filter(
                                geofence::Column::Name
                                    .eq(Value::String(Some(Box::new(name.to_string())))),
                            )
                            .one(conn)
                            .await?;
                        if let Some(geofence) = geofence {
                            Some(geofence.id as u64)
                        } else {
                            None
                        }
                    };
                    if let Some(fence_id) = geofence_id {
                        if let Some(geometry) = feat.geometry.clone() {
                            let geometry = GeoJson::Geometry(geometry).to_json_value();
                            if let Some(geometry) = geometry.as_object() {
                                let geometry = sea_orm::JsonValue::Object(geometry.to_owned());
                                let name = name.to_string();
                                let is_update = existing.get(&format!("{}_{}", name, mode));
                                let update_bool = is_update.is_some();
                                let mut active_model = if let Some(entry) = is_update {
                                    Entity::find_by_id(entry.id)
                                        .one(conn)
                                        .await?
                                        .unwrap()
                                        .into()
                                } else {
                                    ActiveModel {
                                        ..Default::default()
                                    }
                                };
                                active_model.geofence_id = Set(fence_id as u32);
                                active_model.geometry = Set(geometry);
                                active_model.mode = Set(mode);
                                active_model.updated_at = Set(Utc::now());
                                if update_bool {
                                    active_model.update(conn).await?;
                                    inserts_updates.updates += 1;
                                    Ok(())
                                } else {
                                    active_model.name = Set(name);
                                    active_model.created_at = Set(Utc::now());
                                    active_model.insert(conn).await?;
                                    inserts_updates.inserts += 1;
                                    Ok(())
                                }
                            } else {
                                let error =
                                    format!("[ROUTE_SAVE] geometry value is invalid for {}", name);
                                log::warn!("{}", error);
                                Err(DbErr::Custom(error))
                            }
                        } else {
                            let error =
                                format!("[ROUTE_SAVE] geometry value does not exist for {}", name);
                            log::warn!("{}", error);
                            Err(DbErr::Custom(error))
                        }
                    } else {
                        let error =
                            format!("[ROUTE_SAVE] __geofence_id property not found for {}", name);
                        log::warn!("{}", error);
                        Err(DbErr::Custom(error))
                    }
                } else {
                    let error = format!("[ROUTE_SAVE] __mode property not found for {}", name);
                    log::warn!("{}", error);
                    Err(DbErr::Custom(error))
                }
            } else {
                let error = format!(
                    "[ROUTE_SAVE] __name property is not a valid string for {:?}",
                    name
                );
                log::warn!("{}", error);
                Err(DbErr::Custom(error))
            }
        } else {
            let error = format!("[ROUTE_SAVE] __name property not found, {:?}", feat.id);
            log::warn!("{}", error);
            Err(DbErr::Custom(error))
        }
    }

    pub async fn upsert_from_geometry(
        conn: &DatabaseConnection,
        area: GeoFormats,
    ) -> Result<(usize, usize), DbErr> {
        let existing: HashMap<String, RouteNoGeometry> = Query::get_all_no_fences(conn)
            .await?
            .into_iter()
            .map(|model| (format!("{}_{}", model.name, model.mode), model))
            .collect();

        let mut inserts_updates = InsertsUpdates::<ActiveModel> {
            inserts: 0,
            updates: 0,
            to_insert: vec![],
        };

        match area {
            GeoFormats::Feature(feat) => {
                Query::upsert_feature(conn, feat, &existing, &mut inserts_updates).await?
            }
            feat => {
                let fc = match feat {
                    GeoFormats::FeatureCollection(fc) => fc,
                    geometry => geometry.to_collection(None, None),
                };
                for feat in fc.into_iter() {
                    Query::upsert_feature(conn, feat, &existing, &mut inserts_updates).await?
                }
            }
        }

        Ok((inserts_updates.inserts, inserts_updates.updates))
    }

    pub async fn by_geofence(
        db: &DatabaseConnection,
        geofence: String,
    ) -> Result<Vec<Json>, DbErr> {
        match geofence.parse::<u32>() {
            Ok(id) => {
                Entity::find()
                    .order_by(Column::Name, Order::Asc)
                    .filter(Column::GeofenceId.eq(id))
                    .select_only()
                    .column(Column::Id)
                    .column(Column::Name)
                    .column(Column::Mode)
                    .into_json()
                    .all(db)
                    .await
            }
            Err(_) => {
                Entity::find()
                    .order_by(Column::Name, Order::Asc)
                    .filter(Column::Name.eq(geofence))
                    .select_only()
                    .column(Column::Id)
                    .column(Column::Name)
                    .column(Column::Mode)
                    .into_json()
                    .all(db)
                    .await
            }
        }
    }

    pub async fn by_geofence_feature(
        db: &DatabaseConnection,
        geofence_name: String,
        internal: bool,
    ) -> Result<Vec<Feature>, DbErr> {
        let items = match geofence_name.parse::<u32>() {
            Ok(id) => {
                Entity::find()
                    .order_by(Column::Name, Order::Asc)
                    .filter(Column::GeofenceId.eq(id))
                    .all(db)
                    .await?
            }
            Err(_) => {
                Entity::find()
                    .order_by(Column::Name, Order::Asc)
                    .left_join(geofence::Entity)
                    .filter(geofence::Column::Name.eq(geofence_name))
                    .all(db)
                    .await?
            }
        };

        let items: Vec<Feature> = items
            .into_iter()
            .filter_map(|item| item.to_feature(internal).ok())
            .collect();
        Ok(items)
    }

    pub async fn search(db: &DatabaseConnection, search: String) -> Result<Vec<Json>, DbErr> {
        Ok(Entity::find()
            .filter(Column::Name.like(format!("%{}%", search).as_str()))
            .into_json()
            .all(db)
            .await?)
    }

    pub async fn unique_geofence(db: &DatabaseConnection) -> Result<Vec<Json>, ModelError> {
        let items = Entity::find()
            .select_only()
            .column(Column::GeofenceId)
            .distinct()
            .into_model::<OnlyGeofenceId>()
            .all(db)
            .await?;
        let items = geofence::Entity::find()
            .filter(geofence::Column::Id.is_in(items.into_iter().map(|item| item.geofence_id)))
            .select_only()
            .column(geofence::Column::Id)
            .column(geofence::Column::Name)
            .distinct()
            .into_json()
            .all(db)
            .await?;
        Ok(items)
    }
}
