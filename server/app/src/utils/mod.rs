use super::*;

use entity::sea_orm_active_enums::Type;
use geojson::Value;

pub mod clustering;
pub mod drawing;
pub mod normalize;
pub mod response;
pub mod write_debug;
// pub mod routing;

pub fn sql_raw(area: &FeatureCollection) -> String {
    let mut string = "".to_string();
    for feature in area.features.iter() {
        if let Some(geometry) = feature.geometry.as_ref() {
            match geometry.value {
                Value::Polygon(_) | Value::MultiPolygon(_) => {
                    string = format!(
                        "{} {} ST_CONTAINS(ST_GeomFromGeoJSON('{}', 2, 0), POINT(lon, lat))",
                        string,
                        if string.contains("WHERE") {
                            "OR"
                        } else {
                            "WHERE"
                        },
                        feature.geometry.as_ref().unwrap().to_string()
                    );
                }
                _ => {}
            }
        }
    }
    string
}

pub fn get_enum(instance_type: Option<String>) -> Option<Type> {
    match instance_type {
        Some(instance_type) => match instance_type.as_str() {
            "AutoQuest" | "auto_quest" => Some(Type::AutoQuest),
            "CirclePokemon" | "circle_pokemon" => Some(Type::CirclePokemon),
            "CircleSmartPokemon" | "circle_smart_pokemon" => Some(Type::CircleSmartPokemon),
            "CircleRaid" | "circle_raid" => Some(Type::CircleRaid),
            "CircleSmartRaid" | "circle_smart_raid" => Some(Type::CircleSmartRaid),
            "PokemonIv" | "pokemon_iv" => Some(Type::PokemonIv),
            "Leveling" | "leveling" => Some(Type::Leveling),
            _ => None,
        },
        None => None,
    }
}

pub fn get_enum_by_geometry(enum_val: &Value) -> Option<Type> {
    match enum_val {
        Value::Point(_) => Some(Type::Leveling),
        Value::MultiPoint(_) => Some(Type::CirclePokemon),
        Value::Polygon(_) => Some(Type::PokemonIv),
        Value::MultiPolygon(_) => Some(Type::AutoQuest),
        _ => {
            println!("Invalid Geometry Type");
            None
        }
    }
}
