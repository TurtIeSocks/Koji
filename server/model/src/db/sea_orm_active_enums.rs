//! SeaORM Entity. Generated by sea-orm-codegen 0.10.1

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "type")]
pub enum Type {
    #[sea_orm(string_value = "circle_pokemon")]
    CirclePokemon,
    #[sea_orm(string_value = "circle_smart_pokemon")]
    CircleSmartPokemon,
    #[sea_orm(string_value = "circle_raid")]
    CircleRaid,
    #[sea_orm(string_value = "circle_smart_raid")]
    CircleSmartRaid,
    #[sea_orm(string_value = "auto_quest")]
    AutoQuest,
    // This is added manually, re-add it if you run the CLI command
    #[sea_orm(string_value = "circle_quest")]
    CircleQuest,
    #[sea_orm(string_value = "pokemon_iv")]
    PokemonIv,
    #[sea_orm(string_value = "leveling")]
    Leveling,
    #[sea_orm(string_value = "auto_pokemon")]
    AutoPokemon,
    #[sea_orm(string_value = "auto_tth")]
    AutoTth,
    // Only valid in Kōji database
    #[sea_orm(string_value = "unset")]
    Unset,
}

impl Serialize for Type {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Type::AutoPokemon => serializer.serialize_str("auto_pokemon"),
            Type::AutoQuest => serializer.serialize_str("auto_quest"),
            Type::AutoTth => serializer.serialize_str("auto_tth"),
            Type::CirclePokemon => serializer.serialize_str("circle_pokemon"),
            Type::CircleQuest => serializer.serialize_str("circle_quest"),
            Type::CircleRaid => serializer.serialize_str("circle_raid"),
            Type::CircleSmartPokemon => serializer.serialize_str("circle_smart_pokemon"),
            Type::CircleSmartRaid => serializer.serialize_str("circle_smart_raid"),
            Type::Leveling => serializer.serialize_str("leveling"),
            Type::PokemonIv => serializer.serialize_str("pokemon_iv"),
            Type::Unset => serializer.serialize_str("unset"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "category")]
pub enum Category {
    #[sea_orm(string_value = "boolean")]
    Boolean,
    #[sea_orm(string_value = "string")]
    String,
    #[sea_orm(string_value = "number")]
    Number,
    #[sea_orm(string_value = "object")]
    Object,
    #[sea_orm(string_value = "array")]
    Array,
    #[sea_orm(string_value = "database")]
    Database,
    #[sea_orm(string_value = "color")]
    Color,
}

impl Serialize for Category {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Category::Array => serializer.serialize_str("array"),
            Category::Boolean => serializer.serialize_str("boolean"),
            Category::Color => serializer.serialize_str("color"),
            Category::Database => serializer.serialize_str("database"),
            Category::Number => serializer.serialize_str("number"),
            Category::Object => serializer.serialize_str("object"),
            Category::String => serializer.serialize_str("string"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "mode")]
pub enum FenceMode {
    #[sea_orm(string_value = "auto_quest")]
    AutoQuest,
    #[sea_orm(string_value = "auto_pokemon")]
    AutoPokemon,
    #[sea_orm(string_value = "auto_tth")]
    AutoTth,
    #[sea_orm(string_value = "pokemon_iv")]
    PokemonIv,
    #[sea_orm(string_value = "unset")]
    Unset,
}

impl Serialize for FenceMode {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            FenceMode::AutoPokemon => serializer.serialize_str("auto_pokemon"),
            FenceMode::AutoQuest => serializer.serialize_str("auto_quest"),
            FenceMode::AutoTth => serializer.serialize_str("auto_tth"),
            FenceMode::PokemonIv => serializer.serialize_str("pokemon_iv"),
            FenceMode::Unset => serializer.serialize_str("unset"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "mode")]
pub enum RouteMode {
    #[sea_orm(string_value = "circle_pokemon")]
    CirclePokemon,
    #[sea_orm(string_value = "circle_smart_pokemon")]
    CircleSmartPokemon,
    #[sea_orm(string_value = "circle_raid")]
    CircleRaid,
    #[sea_orm(string_value = "circle_smart_raid")]
    CircleSmartRaid,
    #[sea_orm(string_value = "circle_quest")]
    CircleQuest,
    #[sea_orm(string_value = "unset")]
    Unset,
}

impl Serialize for RouteMode {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            RouteMode::CirclePokemon => serializer.serialize_str("circle_pokemon"),
            RouteMode::CircleQuest => serializer.serialize_str("circle_quest"),
            RouteMode::CircleRaid => serializer.serialize_str("circle_raid"),
            RouteMode::CircleSmartPokemon => serializer.serialize_str("circle_smart_pokemon"),
            RouteMode::CircleSmartRaid => serializer.serialize_str("circle_smart_raid"),
            RouteMode::Unset => serializer.serialize_str("unset"),
        }
    }
}
