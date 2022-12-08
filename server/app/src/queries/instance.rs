use migration::Order;
use sea_orm::QueryOrder;

use super::*;

use crate::{
    entity::{instance, sea_orm_active_enums},
    utils::convert::{collection, normalize},
};

pub async fn all(
    conn: &DatabaseConnection,
    instance_type: Option<String>,
) -> Result<Vec<Feature>, DbErr> {
    let instance_type = match instance_type {
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
    };
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
