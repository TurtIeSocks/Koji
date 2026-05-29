//! String / geometry → domain-enum mappers. Return koji-core domain enums
//! (`FenceType`, `Category`); the db layer converts to its sea-orm enums via
//! `enum_bridge!` at the boundary.

use geojson::Value;

use crate::{Category, FenceType};

pub fn get_enum(instance_type: Option<String>) -> FenceType {
    match instance_type {
        Some(instance_type) => match instance_type.as_str() {
            "AutoQuest" | "auto_quest" => FenceType::AutoQuest,
            "CirclePokemon" | "circle_pokemon" => FenceType::CirclePokemon,
            "CircleSmartPokemon" | "circle_smart_pokemon" => FenceType::CircleSmartPokemon,
            "CircleRaid" | "circle_raid" => FenceType::CircleRaid,
            "CircleSmartRaid" | "circle_smart_raid" => FenceType::CircleSmartRaid,
            "CircleStation" | "circle_station" => FenceType::CircleStation,
            "PokemonIv" | "pokemon_iv" => FenceType::PokemonIv,
            "Leveling" | "leveling" => FenceType::Leveling,
            "CircleQuest" | "circle_quest" => FenceType::CircleQuest,
            "AutoTth" | "auto_tth" => FenceType::AutoTth,
            "AutoPokemon" | "auto_pokemon" => FenceType::AutoPokemon,
            _ => FenceType::Unset,
        },
        None => FenceType::Unset,
    }
}

pub fn get_enum_by_geometry(enum_val: &Value) -> FenceType {
    match enum_val {
        Value::Point(_) => FenceType::Leveling,
        Value::MultiPoint(_) => FenceType::CircleSmartPokemon,
        Value::Polygon(_) => FenceType::PokemonIv,
        Value::MultiPolygon(_) => FenceType::AutoQuest,
        _ => {
            log::warn!("Invalid Geometry Type: {}", enum_val.type_name());
            FenceType::Unset
        }
    }
}

pub fn get_category_enum(category: String) -> Category {
    match category.to_lowercase().as_str() {
        "database" => Category::Database,
        "boolean" => Category::Boolean,
        "number" => Category::Number,
        "object" => Category::Object,
        "array" => Category::Array,
        "color" => Category::Color,
        _ => Category::String,
    }
}

pub fn get_enum_by_geometry_string(input: Option<String>) -> Option<FenceType> {
    if let Some(input) = input {
        match input.to_lowercase().as_str() {
            "point" => Some(FenceType::Leveling),
            "multipoint" => Some(FenceType::CirclePokemon),
            "multipolygon" => Some(FenceType::AutoQuest),
            _ => None,
        }
    } else {
        None
    }
}
