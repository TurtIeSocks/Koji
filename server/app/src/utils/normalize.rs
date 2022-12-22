use super::*;

use num_traits::Float;

use crate::{
    entity::{area, instance, sea_orm_active_enums::Type},
    models::{
        scanner::{GenericData, Spawnpoint},
        single_struct::SingleStruct,
        text::TextHelpers,
    },
};

pub fn fort<T>(items: SingleStruct<T>, prefix: &str) -> Vec<GenericData<T>>
where
    T: Float,
{
    items
        .into_iter()
        .enumerate()
        .map(|(i, item)| GenericData::new(format!("{}{}", prefix, i), item.lat, item.lon))
        .collect()
}

pub fn spawnpoint<T>(items: Vec<Spawnpoint<T>>) -> Vec<GenericData<T>>
where
    T: Float,
{
    items
        .into_iter()
        .enumerate()
        .map(|(i, item)| {
            GenericData::new(
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

pub fn instance(instance: instance::Model) -> Feature {
    instance
        .data
        .parse_scanner_instance(Some(instance.name), Some(&instance.r#type))
}

pub fn area(areas: Vec<area::Model>) -> Vec<Feature> {
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
