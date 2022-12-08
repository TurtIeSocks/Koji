use std::collections::HashMap;

use migration::{Expr, Order};
use sea_orm::{QueryOrder, Set};
use serde_json::{json, Value};

use super::*;

use crate::{
    entity::{instance, sea_orm_active_enums},
    utils::convert::{collection, normalize, vector},
};

fn get_enum(instance_type: Option<String>) -> Option<sea_orm_active_enums::Type> {
    match instance_type {
        Some(instance_type) => match instance_type.as_str() {
            "AutoQuest" | "auto_quest" => Some(sea_orm_active_enums::Type::AutoQuest),
            "CirclePokemon" | "circle_pokemon" => Some(sea_orm_active_enums::Type::CirclePokemon),
            "CircleSmartPokemon" | "circle_smart_pokemon" => {
                Some(sea_orm_active_enums::Type::CircleSmartPokemon)
            }
            "CircleRaid" | "circle_raid" => Some(sea_orm_active_enums::Type::CircleRaid),
            "CircleSmartRaid" | "circle_smart_raid" => {
                Some(sea_orm_active_enums::Type::CircleSmartRaid)
            }
            "PokemonIv" | "pokemon_iv" => Some(sea_orm_active_enums::Type::PokemonIv),
            "Leveling" | "leveling" => Some(sea_orm_active_enums::Type::Leveling),
            _ => None,
        },
        None => None,
    }
}

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
        Ok(collection::from_feature(normalize::instance(items)))
    } else {
        Err(DbErr::Custom("Instance not found".to_string()))
    }
}

struct Instance {
    name: String,
    // r#type: sea_orm_active_enums::Type,
    data: HashMap<String, Value>,
}

pub async fn save(
    conn: &DatabaseConnection,
    area: FeatureCollection,
) -> Result<(usize, usize), DbErr> {
    let existing = instance::Entity::find().all(conn).await?;
    let existing: Vec<Instance> = existing
        .into_iter()
        .map(|x| Instance {
            name: x.name,
            data: serde_json::from_str(&x.data).unwrap(),
        })
        .collect();

    let mut inserts: Vec<instance::ActiveModel> = vec![];
    let mut update_len = 0;
    // let mut errors: Vec<String> = vec![];

    for feat in area.into_iter() {
        if let Some(name) = feat.property("name") {
            if let Some(name) = name.as_str() {
                let area = vector::from_geometry(feat.geometry.clone().unwrap());
                let new_area = json!(area);
                // let mut new_area = from_str(&new_area).unwrap();
                let name = name.to_string();
                let is_update = existing.iter().find(|entry| entry.name == name);

                if let Some(entry) = is_update {
                    // let mut entry = entry.clone();
                    // entry
                    //     .data
                    //     .entry("area".to_string())
                    // .and_modify(|x| {
                    //     x = &new_area;
                    // })
                    // .or_insert(new_area);
                    instance::Entity::update_many()
                        .col_expr(
                            instance::Column::Data,
                            Expr::value(json!(entry.data).to_string()),
                        )
                        .filter(instance::Column::Name.eq(entry.name.to_string()))
                        .exec(conn)
                        .await?;
                    update_len += 1;
                } else {
                    inserts.push(instance::ActiveModel {
                        name: Set(name.to_string()),
                        data: Set(json!({ "area": new_area }).to_string()),
                        ..Default::default()
                    })
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
