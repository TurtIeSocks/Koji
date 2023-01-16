use super::*;

use crate::{
    api::text::TextHelpers,
    db::{sea_orm_active_enums::Type, AreaRef, NameTypeId},
};

pub fn fort<T>(items: api::single_struct::SingleStruct<T>, prefix: &str) -> Vec<db::GenericData<T>>
where
    T: Float,
{
    items
        .into_iter()
        .enumerate()
        .map(|(i, item)| db::GenericData::new(format!("{}{}", prefix, i), item.lat, item.lon))
        .collect()
}

pub fn spawnpoint<T>(items: Vec<db::Spawnpoint<T>>) -> Vec<db::GenericData<T>>
where
    T: Float,
{
    items
        .into_iter()
        .enumerate()
        .map(|(i, item)| {
            db::GenericData::new(
                format!(
                    "{}{}",
                    if item.despawn_sec.is_some() { "v" } else { "u" },
                    i
                ),
                item.lat,
                item.lon,
            )
        })
        .collect()
}

pub fn instance(instance: db::instance::Model) -> Feature {
    instance
        .data
        .parse_scanner_instance(Some(instance.name), Some(&instance.r#type))
}

pub fn area(areas: Vec<db::area::Model>) -> Vec<Feature> {
    let mut normalized = Vec::<Feature>::new();

    let mut to_feature = |fence: Option<String>, name: String, category: &str| -> String {
        if let Some(fence) = fence {
            if !fence.is_empty() {
                normalized.push(fence.parse_scanner_instance(
                    Some(name.to_string()),
                    Some(match category {
                        "Fort" => &Type::CircleRaid,
                        "Quest" => &Type::ManualQuest,
                        "Pokemon" => &Type::CirclePokemon,
                        _ => &Type::AutoQuest,
                    }),
                ));
            }
        }
        name
    };
    for area in areas.into_iter() {
        let name = to_feature(area.geofence, area.name, "Fence");
        let name = to_feature(area.fort_mode_route, name, "Fort");
        let name = to_feature(area.quest_mode_route, name, "Quest");
        to_feature(area.pokemon_mode_route, name, "Pokemon");
    }
    normalized
}

pub fn area_ref(areas: Vec<AreaRef>) -> Vec<NameTypeId> {
    let mut normalized = Vec::<NameTypeId>::new();

    for area in areas.into_iter() {
        if area.has_fort {
            normalized.push(NameTypeId {
                id: area.id,
                name: area.name.clone(),
                r#type: Some(Type::CircleRaid),
            });
        }
        if area.has_geofence {
            normalized.push(NameTypeId {
                id: area.id,
                name: area.name.clone(),
                r#type: Some(Type::AutoQuest),
            });
        }
        if area.has_pokemon {
            normalized.push(NameTypeId {
                id: area.id,
                name: area.name.clone(),
                r#type: Some(Type::CirclePokemon),
            });
        }
        if area.has_quest {
            normalized.push(NameTypeId {
                id: area.id,
                name: area.name,
                r#type: Some(Type::ManualQuest),
            });
        }
    }
    normalized
}
