use super::*;

use crate::{
    entities::{instance, sea_orm_active_enums},
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
    let items = if instance_type.is_some() {
        instance::Entity::find()
            .filter(instance::Column::Type.eq(instance_type.unwrap()))
            .all(conn)
            .await?
    } else {
        instance::Entity::find().all(conn).await?
    };
    Ok(items
        .iter()
        .map(|item| normalize::instance(item.clone()))
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
    if items.is_some() {
        Ok(collection::from_feature(normalize::instance(
            items.unwrap(),
        )))
    } else {
        Err(DbErr::Custom("Instance not found".to_string()))
    }
}
