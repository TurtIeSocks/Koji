use geojson::{Feature, FeatureCollection};
use num_traits::Float;

use crate::entities::{area, instance, sea_orm_active_enums::Type};
use crate::models::api::{AreaInput, ReturnType};
use crate::models::scanner::{GenericData, LatLon, TrimmedSpawn};
use crate::utils;

use super::{collection, feature};

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

pub fn instance(instance: instance::Model) -> Feature {
    utils::parse_text(
        instance.data.as_str(),
        Some(instance.name),
        Some(instance.r#type),
    )
}

pub fn area(areas: Vec<area::Model>) -> Vec<Feature> {
    let mut normalized = Vec::<Feature>::new();

    let mut to_feature = |fence: Option<String>, name: String, modifier: &str| -> String {
        if fence.is_some() && !fence.clone().unwrap().is_empty() {
            normalized.push(utils::parse_text(
                fence.unwrap().as_str(),
                Some(format!("{}_{}", name, modifier)),
                Some(match modifier {
                    "Fort" => Type::CircleRaid,
                    "Quest" => Type::ManualQuest,
                    "Pokemon" => Type::CirclePokemon,
                    _ => Type::AutoQuest,
                }),
            ));
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

pub fn area_input(area: Option<AreaInput>) -> (FeatureCollection, ReturnType) {
    if area.is_some() {
        let area = area.unwrap();
        match area {
            AreaInput::Text(area) => (
                collection::from_feature(feature::from_text(area.as_str(), None)),
                ReturnType::Text,
            ),
            AreaInput::SingleArray(area) => (
                collection::from_feature(feature::from_single_vector(area, None)),
                ReturnType::SingleArray,
            ),
            AreaInput::MultiArray(area) => (
                collection::from_feature(feature::from_multi_vector(area, None)),
                ReturnType::MultiArray,
            ),
            AreaInput::SingleStruct(area) => (
                collection::from_feature(feature::from_single_struct(area, None)),
                ReturnType::SingleStruct,
            ),
            AreaInput::MultiStruct(area) => (
                collection::from_feature(feature::from_multi_struct(area, None)),
                ReturnType::MultiStruct,
            ),
            AreaInput::Feature(area) => (collection::from_feature(area), ReturnType::Feature),
            AreaInput::FeatureCollection(area) => (area, ReturnType::FeatureCollection),
        }
    } else {
        (
            FeatureCollection {
                bbox: None,
                foreign_members: None,
                features: vec![],
            },
            ReturnType::SingleArray,
        )
    }
}
