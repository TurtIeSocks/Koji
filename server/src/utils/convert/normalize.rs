use std::str::FromStr;

use num_traits::Float;

use crate::entities::{area, instance, sea_orm_active_enums::Type};
use crate::models::api::{AreaInput, ReturnType};
use crate::models::scanner::{GenericData, GenericInstance, LatLon, TrimmedSpawn};
use crate::utils::convert::arrays;

pub fn fort<T>(items: Vec<LatLon<T>>, gym: bool) -> Vec<GenericData<T>>
where
    T: Float,
{
    items
        .into_iter()
        .enumerate()
        .map(|(i, item)| {
            GenericData::new(
                format!("{}{}", if gym { "g" } else { "p" }, i),
                item.lat,
                item.lon,
            )
        })
        .collect()
}

pub fn spawnpoint<T>(items: Vec<TrimmedSpawn<T>>) -> Vec<GenericData<T>>
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

pub fn instance<T>(instance: instance::Model) -> GenericInstance<T>
where
    T: Float + serde::de::DeserializeOwned,
{
    GenericInstance {
        name: instance.name,
        r#type: instance.r#type.clone(),
        data: match instance.r#type {
            Type::AutoQuest | Type::PokemonIv => {
                arrays::parse_multi_polygon::<T>(instance.data.as_str())
            }
            _ => arrays::parse_single_polygon::<T>(instance.data.as_str()),
        },
    }
}

pub fn area<T>(areas: Vec<area::Model>) -> Vec<GenericInstance<T>>
where
    T: Float + FromStr,
{
    let mut normalized = Vec::<GenericInstance<T>>::new();

    for other_area in areas.into_iter() {
        if other_area.geofence.is_some() && !other_area.geofence.clone().unwrap().is_empty() {
            let data = arrays::parse_flat_text(other_area.geofence.clone().unwrap().as_str());
            normalized.push(GenericInstance {
                name: other_area.name.clone(),
                r#type: Type::AutoQuest,
                data,
            });
        }
        if other_area.fort_mode_route.is_some()
            && !other_area.fort_mode_route.clone().unwrap().is_empty()
        {
            let data =
                arrays::parse_flat_text(other_area.fort_mode_route.clone().unwrap().as_str());
            normalized.push(GenericInstance {
                name: other_area.name.clone(),
                r#type: Type::CircleRaid,
                data,
            });
        }
        if other_area.quest_mode_route.is_some()
            && !other_area.quest_mode_route.clone().unwrap().is_empty()
        {
            let data =
                arrays::parse_flat_text(other_area.quest_mode_route.clone().unwrap().as_str());
            normalized.push(GenericInstance {
                name: other_area.name.clone(),
                r#type: Type::ManualQuest,
                data,
            });
        }
        if other_area.pokemon_mode_route.is_some()
            && !other_area.pokemon_mode_route.clone().unwrap().is_empty()
        {
            let data =
                arrays::parse_flat_text(other_area.pokemon_mode_route.clone().unwrap().as_str());
            normalized.push(GenericInstance {
                name: other_area.name,
                r#type: Type::CirclePokemon,
                data,
            });
        }
    }
    normalized
}

pub fn area_input(area: AreaInput) -> (Vec<Vec<[f64; 2]>>, ReturnType) {
    match area {
        AreaInput::Text(area) => (arrays::parse_flat_text(area.as_str()), ReturnType::Text),
        AreaInput::SingleArray(area) => (vec![area], ReturnType::SingleArray),
        AreaInput::MultiArray(area) => (area, ReturnType::MultiArray),
        AreaInput::SingleStruct(area) => {
            (vec![arrays::coord_to_array(area)], ReturnType::SingleStruct)
        }
        AreaInput::MultiStruct(area) => (
            area.into_iter().map(arrays::coord_to_array).collect(),
            ReturnType::MultiStruct,
        ),
    }
}
