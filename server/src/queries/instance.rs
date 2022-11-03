use super::*;
use crate::entities::{instance, sea_orm_active_enums};
// use crate::db::{schema::instance::dsl::*, sql_types::InstanceType};
// use crate::models::scanner::Instance;

pub async fn query_all_instances(
    conn: &DatabaseConnection,
    instance_type: Option<String>,
) -> Result<Vec<instance::Model>, DbErr> {
    let instance_type = match instance_type {
        Some(instance_type) => match instance_type.as_str() {
            "AutoQuest" => Some(sea_orm_active_enums::Type::AutoQuest),
            "auto_quest" => Some(sea_orm_active_enums::Type::AutoQuest),
            "CirclePokemon" => Some(sea_orm_active_enums::Type::CirclePokemon),
            "circle_pokemon" => Some(sea_orm_active_enums::Type::CirclePokemon),
            "CircleSmartPokemon" => Some(sea_orm_active_enums::Type::CircleSmartPokemon),
            "circle_smart_pokemon" => Some(sea_orm_active_enums::Type::CircleSmartPokemon),
            "CircleRaid" => Some(sea_orm_active_enums::Type::CircleRaid),
            "circle_raid" => Some(sea_orm_active_enums::Type::CircleRaid),
            "CircleSmartRaid" => Some(sea_orm_active_enums::Type::CircleSmartRaid),
            "circle_smart_raid" => Some(sea_orm_active_enums::Type::CircleSmartRaid),
            "PokemonIv" => Some(sea_orm_active_enums::Type::PokemonIv),
            "pokemon_iv" => Some(sea_orm_active_enums::Type::PokemonIv),
            "Leveling" => Some(sea_orm_active_enums::Type::Leveling),
            "leveling" => Some(sea_orm_active_enums::Type::Leveling),
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
    Ok(items)
}

pub async fn query_instance_route(
    conn: &DatabaseConnection,
    instance_name: &String,
) -> Result<instance::Model, DbErr> {
    let items = instance::Entity::find()
        .filter(instance::Column::Name.contains(instance_name))
        .one(conn)
        .await?;
    if items.is_some() {
        Ok(items.unwrap())
    } else {
        Err(DbErr::Custom("Instance not found".to_string()))
    }
}
