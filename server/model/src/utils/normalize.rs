use super::*;

use crate::{api::text::TextHelpers, db::sea_orm_active_enums::Type};

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
