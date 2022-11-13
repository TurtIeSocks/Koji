use crate::entities::sea_orm_active_enums::Type;
use crate::models::scanner::GenericInstance;
use geojson::{feature::Id, Feature, FeatureCollection, Geometry, JsonObject, Value};

pub fn arr_to_fc(areas: Vec<GenericInstance>) -> FeatureCollection {
    FeatureCollection {
        bbox: None,
        foreign_members: None,
        features: areas
            .into_iter()
            .enumerate()
            .map(|(i, area)| Feature {
                id: Some(Id::String(i.to_string())),
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
            })
            .collect(),
    }
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
