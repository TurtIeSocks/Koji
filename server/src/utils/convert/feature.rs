use crate::models::{api::ArrayType, scanner::LatLon};
use geojson::{Geometry, Value};

use super::*;

fn multi_polygon(area: Vec<Vec<[f64; 2]>>) -> Value {
    Value::MultiPolygon(vec![area
        .into_iter()
        .map(|poly| {
            ensure_first_last(poly)
                .into_iter()
                .map(|[lat, lon]| vec![lon, lat])
                .collect()
        })
        .collect()])
}

fn polygon(area: Vec<[f64; 2]>) -> Value {
    Value::Polygon(vec![ensure_first_last(area)
        .into_iter()
        .map(|[lat, lon]| vec![lon, lat])
        .collect()])
}

fn multi_point(area: Vec<[f64; 2]>) -> Value {
    Value::MultiPoint(area.into_iter().map(|[lat, lon]| vec![lon, lat]).collect())
}

fn point([lat, lon]: [f64; 2]) -> Value {
    Value::Point(vec![lon, lat])
}

fn value_router(area: ArrayType, enum_type: Option<Type>) -> Value {
    if enum_type.is_some() {
        match area {
            ArrayType::S(area) => match enum_type.unwrap() {
                Type::CirclePokemon
                | Type::CircleSmartPokemon
                | Type::CircleRaid
                | Type::CircleSmartRaid
                | Type::ManualQuest => multi_point(area),
                Type::AutoQuest | Type::PokemonIv => polygon(area),
                Type::Leveling => point(area[0]),
            },
            ArrayType::M(area) => multi_polygon(area),
        }
    } else {
        match area {
            ArrayType::S(area) => polygon(area),
            ArrayType::M(area) => multi_polygon(area),
        }
    }
}

pub fn get_feature(area: ArrayType, enum_type: Option<Type>) -> Feature {
    Feature {
        id: None,
        bbox: None,
        geometry: Some(Geometry {
            bbox: None,
            foreign_members: None,
            value: value_router(area, enum_type),
        }),
        foreign_members: None,
        properties: None,
    }
}

pub fn from_single_vector(area: Vec<[f64; 2]>, enum_type: Option<Type>) -> Feature {
    get_feature(ArrayType::S(area), enum_type)
}

pub fn from_multi_vector(area: Vec<Vec<[f64; 2]>>, enum_type: Option<Type>) -> Feature {
    get_feature(ArrayType::M(area), enum_type)
}

pub fn from_text(area: &str, enum_type: Option<Type>) -> Feature {
    get_feature(ArrayType::S(vector::from_text(area)), enum_type)
}

pub fn from_single_point(area: LatLon) -> Feature {
    get_feature(
        ArrayType::S(vector::from_struct(vec![area])),
        Some(Type::Leveling),
    )
}

pub fn from_single_struct(area: Vec<LatLon>, enum_type: Option<Type>) -> Feature {
    get_feature(ArrayType::S(vector::from_struct(area)), enum_type)
}

pub fn from_multi_struct(area: Vec<Vec<LatLon>>, enum_type: Option<Type>) -> Feature {
    get_feature(
        ArrayType::M(area.into_iter().map(|a| vector::from_struct(a)).collect()),
        enum_type,
    )
}
