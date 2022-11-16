use crate::entities::sea_orm_active_enums::Type;
use crate::models::scanner::GenericInstance;
use geojson::{Feature, FeatureCollection, Geometry, JsonObject, Value};

pub fn instance_to_fc(areas: Vec<GenericInstance>) -> FeatureCollection {
    FeatureCollection {
        bbox: None,
        foreign_members: None,
        features: areas.into_iter().map(instance_to_feature).collect(),
    }
}

pub fn instance_to_feature(area: GenericInstance) -> Feature {
    Feature {
        id: None,
        bbox: None,
        geometry: Some(Geometry {
            bbox: None,
            foreign_members: None,
            value: match area.r#type {
                Type::AutoQuest | Type::PokemonIv => Value::Polygon(
                    area.data
                        .into_iter()
                        .map(|polygon| {
                            polygon
                                .into_iter()
                                .map(|point| vec![point[1] as f64, point[0] as f64])
                                .collect()
                        })
                        .collect::<Vec<_>>(),
                ),
                _ => Value::MultiPoint(
                    area.data[0]
                        .iter()
                        .map(|point| vec![point[1] as f64, point[0] as f64])
                        .collect::<Vec<_>>(),
                ),
            },
        }),
        foreign_members: None,
        properties: Some(get_properties(area.name, area.r#type)),
    }
}

pub fn fc_to_vec(fc: FeatureCollection) -> Vec<Vec<[f64; 2]>> {
    let mut return_value = Vec::<Vec<[f64; 2]>>::new();

    for feature in fc.features.into_iter() {
        if feature.geometry.is_some() {
            return_value.push(feature_to_vec(feature));
        }
    }
    return_value
}

pub fn feature_to_vec(feature: Feature) -> Vec<[f64; 2]> {
    let mut temp_arr = Vec::<[f64; 2]>::new();
    match feature.geometry.unwrap().value {
        Value::MultiPolygon(geometry) => {
            for poly in geometry.into_iter() {
                for point in poly.into_iter() {
                    for p in point.into_iter() {
                        if p.len() == 2 {
                            temp_arr.push([p[1], p[0]]);
                        }
                    }
                }
            }
        }
        Value::Polygon(geometry) => {
            for poly in geometry.into_iter() {
                for point in poly.into_iter() {
                    if point.len() == 2 {
                        temp_arr.push([point[1], point[0]]);
                    }
                }
            }
        }
        Value::MultiPoint(geometry) => {
            for point in geometry.into_iter() {
                if point.len() == 2 {
                    temp_arr.push([point[1], point[0]]);
                }
            }
        }
        Value::Point(geometry) => {
            if geometry.len() == 2 {
                temp_arr.push([geometry[1], geometry[0]]);
            }
        }
        _ => {}
    }
    temp_arr
}

fn get_properties(name: String, enum_type: Type) -> JsonObject {
    let mut properties = JsonObject::new();
    properties.insert("name".to_string(), name.into());
    properties.insert("type".to_string(), enum_type.to_string().into());
    match enum_type {
        Type::CirclePokemon | Type::CircleSmartPokemon => {
            properties.insert("radius".to_string(), 70.into());
        }
        Type::CircleRaid | Type::CircleSmartRaid => {
            properties.insert("radius".to_string(), 700.into());
        }
        Type::ManualQuest => {
            properties.insert("radius".to_string(), 80.into());
        }
        _ => {}
    }
    properties
}

// pub fn
