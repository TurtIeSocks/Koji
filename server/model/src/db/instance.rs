//! SeaORM Entity. Generated by sea-orm-codegen 0.10.1

use std::collections::HashMap;

use crate::{
    api::{
        text::TextHelpers, GeoFormats, ToCollection, ToMultiStruct, ToMultiVec, ToPointStruct,
        ToSingleStruct, ToSingleVec,
    },
    error::ModelError,
    utils::get_mode_acronym,
};

use super::{
    sea_orm_active_enums::Type, utils, Feature, InsertsUpdates, NameTypeId, Order, QueryOrder,
    RdmInstanceArea, ToFeatureFromModel,
};

use sea_orm::{
    entity::prelude::*, sea_query::Expr, DbBackend, FromQueryResult, QuerySelect, Set, Statement,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "instance")]
pub struct Model {
    #[sea_orm(primary_key)]
    #[serde(skip_deserializing)]
    pub id: u32,
    #[sea_orm(unique)]
    pub name: String,
    pub r#type: Type,
    #[sea_orm(column_type = "custom(\"LONGTEXT\")")]
    pub data: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

#[derive(Debug, Serialize, FromQueryResult)]
pub struct WithGeoType {
    pub id: u32,
    pub name: String,
    pub mode: Type,
    pub geo_type: String,
}

impl ToFeatureFromModel for Model {
    fn to_feature(self, _args: bool) -> Result<Feature, ModelError> {
        let Self {
            id,
            name,
            r#type: mode,
            data,
            ..
        } = self;
        let mut feature = data.parse_scanner_instance(Some(name.clone()), Some(mode.clone()));
        feature.id = Some(geojson::feature::Id::String(format!(
            "{}__{}__SCANNER",
            id,
            mode.to_value(),
        )));
        feature.set_property("__name", name);
        feature.set_property("__mode", mode.to_value());
        feature.set_property("__id", id);
        Ok(feature)
    }
}

pub struct Query;

impl Query {
    pub async fn all(
        conn: &DatabaseConnection,
        instance_type: Option<String>,
    ) -> Result<Vec<NameTypeId>, DbErr> {
        let instance_type = utils::get_enum(instance_type);
        if instance_type != Type::Unset && instance_type != Type::CircleQuest {
            Entity::find()
                .filter(Column::Type.eq(instance_type))
                .select_only()
                .column(Column::Name)
                .column_as(Column::Type, "mode")
                .order_by(Column::Name, Order::Asc)
                .into_model::<NameTypeId>()
                .all(conn)
                .await
        } else {
            Entity::find()
                .select_only()
                .column(Column::Id)
                .column(Column::Name)
                .column_as(Column::Type, "mode")
                .order_by(Column::Name, Order::Asc)
                .into_model::<NameTypeId>()
                .all(conn)
                .await
        }
    }

    pub async fn get_json_cache(db: &DatabaseConnection) -> Result<Vec<sea_orm::JsonValue>, DbErr> {
        let entries = Entity::find()
            .from_raw_sql(Statement::from_sql_and_values(
                DbBackend::MySql,
                r#"SELECT id, name, type AS mode,
                        CASE 
                            WHEN type = 'leveling' THEN 'Point'
                            WHEN type LIKE 'circle_%' THEN 'MultiPoint'
                            ELSE 'MultiPolygon'
                        END AS geo_type
                        FROM instance
                        ORDER BY name
                    "#,
                vec![],
            ))
            .into_model::<WithGeoType>()
            .all(db)
            .await?;
        Ok(entries.into_iter().map(|entry| json!(entry)).collect())
    }

    pub async fn feature_from_name(
        conn: &DatabaseConnection,
        name: &String,
    ) -> Result<Feature, ModelError> {
        let item = Entity::find()
            .filter(Column::Name.eq(name.trim().to_string()))
            .one(conn)
            .await?;
        if let Some(item) = item {
            item.to_feature(false)
        } else {
            Err(ModelError::Custom("Instance not found".to_string()))
        }
    }

    pub async fn feature(conn: &DatabaseConnection, id: u32) -> Result<Feature, ModelError> {
        let item = Entity::find_by_id(id).one(conn).await?;
        if let Some(item) = item {
            item.to_feature(false)
        } else {
            Err(ModelError::Custom("Instance not found".to_string()))
        }
    }

    async fn get_by_name(conn: &DatabaseConnection, name: String) -> Result<Option<Model>, DbErr> {
        Entity::find().filter(Column::Name.eq(name)).one(conn).await
    }

    async fn get_default(
        conn: &DatabaseConnection,
        short: &String,
        mode: &Type,
    ) -> Result<HashMap<String, Value>, DbErr> {
        if let Some(default_model) = Query::get_by_name(conn, format!("Default_{}", short)).await? {
            log::info!("Found default for Default_{}", short);
            match serde_json::from_str(&default_model.data) {
                Ok(defaults) => Ok(defaults),
                Err(err) => Err(DbErr::Custom(err.to_string())),
            }
        } else if let Some(default_model) =
            Query::get_by_name(conn, format!("Default_{}", mode.to_value())).await?
        {
            log::info!("Found default for Default_{}", mode.to_value());
            match serde_json::from_str(&default_model.data) {
                Ok(defaults) => Ok(defaults),
                Err(err) => Err(DbErr::Custom(err.to_string())),
            }
        } else {
            Ok(HashMap::new())
        }
    }

    async fn upsert_feature(
        conn: &DatabaseConnection,
        feat: Feature,
        existing: &HashMap<String, Model>,
        inserts_updates: &mut InsertsUpdates<ActiveModel>,
    ) -> Result<(), DbErr> {
        if let Some(name) = feat.property("__name") {
            if let Some(name) = name.as_str() {
                let mode = if let Some(instance_type) = feat.property("__mode") {
                    if let Some(instance_type) = instance_type.as_str() {
                        utils::get_enum(Some(instance_type.to_string()))
                    } else {
                        utils::get_enum_by_geometry(&feat.geometry.as_ref().unwrap().value)
                    }
                } else {
                    utils::get_enum_by_geometry(&feat.geometry.as_ref().unwrap().value)
                };
                let area = match mode {
                    Type::CirclePokemon
                    | Type::CircleSmartPokemon
                    | Type::CircleRaid
                    | Type::CircleSmartRaid
                    | Type::CircleQuest => {
                        RdmInstanceArea::Single(feat.clone().to_single_vec().to_single_struct())
                    }
                    Type::Leveling => {
                        RdmInstanceArea::Leveling(feat.clone().to_single_vec().to_struct())
                    }
                    Type::AutoQuest | Type::PokemonIv | Type::AutoPokemon | Type::AutoTth => {
                        RdmInstanceArea::Multi(feat.clone().to_multi_vec().to_multi_struct())
                    }
                    Type::Unset => return Err(DbErr::Custom("Instance type not set".to_string())),
                };
                let new_area = json!(area);
                // let id = if let Some(id) = feat.property("__id") {
                //     id.as_u64()
                // } else {
                //     log::info!(
                //         "ID not found, attempting to save by name ({}) and mode ({})",
                //         name,
                //         mode
                //     );
                //     None
                // };
                // log::warn!("ID: {:?}", id);
                // if let Some(id) = id {
                //     let model = Entity::find_by_id(id as u32).one(conn).await?;
                //     if let Some(model) = model {
                //         let mut data: HashMap<String, Value> =
                //             serde_json::from_str(&model.data).unwrap();
                //         data.insert("area".to_string(), new_area);

                //         if let Ok(data) = serde_json::to_string(&data) {
                //             let mut model: ActiveModel = model.into();
                //             model.data = Set(data);
                //             match model.update(conn).await {
                //                 Ok(_) => {
                //                     log::info!("Successfully updated {}", id);
                //                     Ok(())
                //                 }
                //                 Err(err) => {
                //                     let error = format!("Unable to update {}: {:?}", id, err);
                //                     log::error!("{}", error);
                //                     Err(DbErr::Custom(error))
                //                 }
                //             }
                //         } else {
                //             let error = format!("Unable to serialize json: {:?}", data);
                //             log::error!("{}", error);
                //             Err(DbErr::Custom(error))
                //         }
                //     } else {
                //         let error = format!(
                //             "Found an ID but was unable to find the record in the db: {}",
                //             id
                //         );
                //         log::error!("{}", error);
                //         Err(DbErr::Custom(error))
                //     }
                // } else {
                let name = name.to_string();
                let is_update = existing.get(&name);
                let short = get_mode_acronym(Some(&mode.to_value()));
                if let Some(entry) = is_update {
                    if entry.r#type == mode {
                        let mut data: HashMap<String, Value> =
                            serde_json::from_str(&entry.data).unwrap();
                        data.insert("area".to_string(), new_area);

                        Entity::update_many()
                            .col_expr(Column::Data, Expr::value(json!(data).to_string()))
                            .filter(Column::Id.eq(entry.id))
                            .exec(conn)
                            .await?;
                        inserts_updates.updates += 1;
                        Ok(())
                    } else if let Some(actual_entry) = existing.get(&format!("{}_{}", name, short))
                    {
                        let mut data: HashMap<String, Value> =
                            serde_json::from_str(&actual_entry.data).unwrap();
                        data.insert("area".to_string(), new_area);

                        Entity::update_many()
                            .col_expr(Column::Data, Expr::value(json!(data).to_string()))
                            .filter(Column::Name.eq(actual_entry.name.to_string()))
                            .exec(conn)
                            .await?;
                        inserts_updates.updates += 1;
                        Ok(())
                    } else {
                        let mut active_model = ActiveModel {
                            name: Set(format!("{}_{}", name, short)),
                            ..Default::default()
                        };
                        let mut data = Query::get_default(conn, &short, &mode).await?;
                        data.insert("area".to_string(), new_area);
                        active_model.data = Set(json!(data).to_string());
                        active_model.r#type = Set(mode);

                        inserts_updates.inserts += 1;
                        inserts_updates.to_insert.push(active_model);
                        Ok(())
                    }
                } else {
                    let mut active_model = ActiveModel {
                        name: Set(name.to_string()),
                        ..Default::default()
                    };
                    let mut data = Query::get_default(conn, &short, &mode).await?;
                    data.insert("area".to_string(), new_area);
                    active_model.data = Set(json!(data).to_string());
                    active_model.r#type = Set(mode);

                    inserts_updates.inserts += 1;
                    inserts_updates.to_insert.push(active_model);
                    Ok(())
                }
                // }
            } else {
                let error = format!(
                    "Name property is not a properly formatted string | {}",
                    name
                );
                log::warn!("{}", error);
                Err(DbErr::Custom(error))
            }
        } else {
            let error = format!(
                "Name not found, unable to save feature {:?}",
                feat.properties
            );
            log::warn!("{}", error);
            Err(DbErr::Custom(error))
        }
    }

    pub async fn upsert_from_geometry(
        conn: &DatabaseConnection,
        area: GeoFormats,
        _auto_mode: bool,
    ) -> Result<(usize, usize), DbErr> {
        let existing: HashMap<String, Model> = Entity::find()
            .all(conn)
            .await?
            .into_iter()
            .map(|model| (model.name.to_string(), model))
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
        if !inserts_updates.to_insert.is_empty() {
            Entity::insert_many(inserts_updates.to_insert)
                .exec(conn)
                .await?;
        }
        Ok((inserts_updates.inserts, inserts_updates.updates))
    }
}
