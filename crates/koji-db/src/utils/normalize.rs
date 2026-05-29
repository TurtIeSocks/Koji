use geojson::Feature;
use serde_json::json;

use crate::text::TextHelpers;
use crate::db::{AreaRef, sea_orm_active_enums::Type};

pub fn instance(instance: crate::db::instance::Model) -> Feature {
    instance
        .data
        .parse_scanner_instance(Some(instance.name), Some(instance.r#type.into()))
}

pub fn area(areas: Vec<crate::db::area::Model>) -> Vec<Feature> {
    let mut normalized = Vec::<Feature>::new();

    let mut to_feature = |fence: Option<String>, name: &String, mode: Type| {
        if let Some(fence) = fence {
            if !fence.is_empty() {
                normalized.push(
                    fence.parse_scanner_instance(Some(name.to_string()), Some(mode.into())),
                );
            }
        }
    };
    for area in areas.into_iter() {
        to_feature(area.geofence, &area.name, Type::AutoQuest);
        to_feature(area.fort_mode_route, &area.name, Type::CircleRaid);
        to_feature(area.quest_mode_route, &area.name, Type::CircleQuest);
        to_feature(area.pokemon_mode_route, &area.name, Type::CirclePokemon);
    }
    normalized
}

pub fn area_ref(areas: Vec<AreaRef>) -> Vec<sea_orm::JsonValue> {
    let mut normalized = Vec::<sea_orm::JsonValue>::new();

    for area in areas.into_iter() {
        if area.has_geofence {
            normalized.push(json!({
                "id": area.id,
                "name": area.name,
                "mode": "auto_quest",
                "geo_type": "MultiPolygon",
            }));
        }
        if area.has_fort {
            normalized.push(json!({
                "id": area.id,
                "name": area.name,
                "mode": "circle_raid",
                "geo_type": "MultiPoint",
            }));
        }
        if area.has_pokemon {
            normalized.push(json!({
                "id": area.id,
                "name": area.name,
                "mode": "circle_pokemon",
                "geo_type": "MultiPoint",
            }));
        }
        if area.has_quest {
            normalized.push(json!({
                "id": area.id,
                "name": area.name,
                "mode": "circle_quest",
                "geo_type": "MultiPoint",
            }));
        }
    }
    normalized
}
