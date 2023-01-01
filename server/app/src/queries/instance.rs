use std::collections::HashMap;

use entity::sea_orm_active_enums::Type;
use migration::{Expr, Order};
use models::{scanner::RdmInstanceArea, ToMultiStruct, ToMultiVec, ToSingleStruct};
use sea_orm::{ActiveModelTrait, QueryOrder, Set};
use serde_json::{json, Value};

use super::*;

use crate::{
    entity::instance,
    models::{ToCollection, ToPointStruct, ToSingleVec},
    utils::{get_enum, get_enum_by_geometry, normalize},
};

pub async fn all(
    conn: &DatabaseConnection,
    instance_type: Option<String>,
) -> Result<Vec<Feature>, DbErr> {
    let instance_type = get_enum(instance_type);
    let items = if let Some(instance_type) = instance_type {
        instance::Entity::find()
            .filter(instance::Column::Type.eq(instance_type))
            .order_by(instance::Column::Name, Order::Asc)
            .all(conn)
            .await?
    } else {
        instance::Entity::find()
            .order_by(instance::Column::Name, Order::Asc)
            .all(conn)
            .await?
    };
    Ok(items
        .into_iter()
        .map(|item| normalize::instance(item))
        .collect())
}

pub async fn route(
    conn: &DatabaseConnection,
    instance_name: &String,
) -> Result<FeatureCollection, DbErr> {
    let items = instance::Entity::find()
        .filter(instance::Column::Name.contains(instance_name))
        .one(conn)
        .await?;
    if let Some(items) = items {
        Ok(normalize::instance(items).to_collection(None, None))
    } else {
        Err(DbErr::Custom("Instance not found".to_string()))
    }
}

struct Instance {
    name: String,
    // r#type: Type,
    data: HashMap<String, Value>,
}

pub async fn save(
    conn: &DatabaseConnection,
    area: FeatureCollection,
) -> Result<(usize, usize), DbErr> {
    let existing = instance::Entity::find().all(conn).await?;
    let mut existing: Vec<Instance> = existing
        .into_iter()
        .map(|x| Instance {
            name: x.name,
            // r#type: x.r#type,
            data: serde_json::from_str(&x.data).unwrap(),
        })
        .collect();

    let mut inserts: Vec<instance::ActiveModel> = vec![];
    let mut update_len = 0;

    for feat in area.into_iter() {
        if let Some(name) = feat.property("name") {
            if let Some(name) = name.as_str() {
                let r#type = if let Some(instance_type) = feat.property("type") {
                    if let Some(instance_type) = instance_type.as_str() {
                        get_enum(Some(instance_type.to_string()))
                    } else {
                        get_enum_by_geometry(&feat.geometry.as_ref().unwrap().value)
                    }
                } else {
                    get_enum_by_geometry(&feat.geometry.as_ref().unwrap().value)
                };
                if let Some(r#type) = r#type {
                    let area = match r#type {
                        Type::CirclePokemon
                        | Type::CircleSmartPokemon
                        | Type::CircleRaid
                        | Type::CircleSmartRaid
                        | Type::ManualQuest => {
                            RdmInstanceArea::Single(feat.clone().to_single_vec().to_single_struct())
                        }
                        Type::Leveling => {
                            RdmInstanceArea::Leveling(feat.clone().to_single_vec().to_struct())
                        }
                        Type::AutoQuest | Type::PokemonIv | Type::AutoPokemon | Type::AutoTth => {
                            RdmInstanceArea::Multi(feat.clone().to_multi_vec().to_multi_struct())
                        }
                    };
                    let new_area = json!(area);
                    let name = name.to_string();
                    let is_update = existing.iter_mut().find(|entry| entry.name == name);

                    if let Some(entry) = is_update {
                        entry.data.insert("area".to_string(), new_area);
                        instance::Entity::update_many()
                            .col_expr(
                                instance::Column::Data,
                                Expr::value(json!(entry.data).to_string()),
                            )
                            .col_expr(instance::Column::Type, Expr::value(r#type))
                            .filter(instance::Column::Name.eq(entry.name.to_string()))
                            .exec(conn)
                            .await?;
                        update_len += 1;
                    } else {
                        let mut active_model = instance::ActiveModel {
                            name: Set(name.to_string()),
                            // r#type: Set(r#type),
                            // data: Set(json!({ "area": new_area }).to_string()),
                            ..Default::default()
                        };
                        active_model
                            .set_from_json(json!({
                                "name": name,
                                "type": r#type,
                                "data": json!({ "area": new_area }).to_string(),
                            }))
                            .unwrap();

                        inserts.push(active_model)
                    }
                }
            }
        }
    }
    let insert_len = inserts.len();
    if !inserts.is_empty() {
        instance::Entity::insert_many(inserts).exec(conn).await?;
    }
    Ok((insert_len, update_len))
}
