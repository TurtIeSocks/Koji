use super::vector::from_struct;
use super::*;
use num_traits::Float;

use crate::models::api::DataPointsArg;
use crate::models::{SingleStruct, SingleVec};
use crate::utils::{self, text_test};
use crate::{
    entities::{area, instance, sea_orm_active_enums::Type},
    models::{
        api::ReturnTypeArg,
        scanner::{GenericData, Spawnpoint},
        GeoFormats,
    },
};

use super::collection::Default;
use super::{collection, feature};

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

pub fn data_points(data_points: Option<DataPointsArg>) -> SingleVec {
    if let Some(data_points) = data_points {
        match data_points {
            DataPointsArg::Struct(data_points) => from_struct(data_points),
            DataPointsArg::Array(data_points) => data_points,
        }
    } else {
        vec![]
    }
}

pub fn area_input(area: Option<GeoFormats>) -> (FeatureCollection, ReturnTypeArg) {
    if area.is_some() {
        let area = area.unwrap();
        match area {
            GeoFormats::Text(area) => (
                collection::from_feature(feature::from_text(area.as_str(), None)),
                if text_test(area.as_str()) {
                    ReturnTypeArg::AltText
                } else {
                    ReturnTypeArg::Text
                },
            ),
            GeoFormats::SingleArray(area) => (
                collection::from_feature(feature::from_single_vector(area, None)),
                ReturnTypeArg::SingleArray,
            ),
            GeoFormats::MultiArray(area) => (
                collection::from_feature(feature::from_multi_vector(area, None)),
                ReturnTypeArg::MultiArray,
            ),
            GeoFormats::SingleStruct(area) => (
                collection::from_feature(feature::from_single_struct(area, None)),
                ReturnTypeArg::SingleStruct,
            ),
            GeoFormats::MultiStruct(area) => (
                collection::from_feature(feature::from_multi_struct(area, None)),
                ReturnTypeArg::MultiStruct,
            ),
            GeoFormats::Feature(area) => (collection::from_feature(area), ReturnTypeArg::Feature),
            GeoFormats::FeatureVec(areas) => {
                (collection::from_features(areas), ReturnTypeArg::FeatureVec)
            }
            GeoFormats::FeatureCollection(area) => (area, ReturnTypeArg::FeatureCollection),
        }
    } else {
        (FeatureCollection::default(), ReturnTypeArg::SingleArray)
    }
}
