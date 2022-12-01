use num_traits::Float;

use super::*;
use crate::entities::sea_orm_active_enums::Type;
use crate::models::{
    api::ReturnTypeArg,
    scanner::{InstanceParsing, RdmInstanceArea},
};

pub mod convert;
pub mod drawing;
pub mod response;
// pub mod routing;

trait SetOptions {
    fn add_instance_properties(&mut self, name: Option<String>, enum_type: Option<&Type>);
}

impl SetOptions for Feature {
    fn add_instance_properties(&mut self, name: Option<String>, enum_type: Option<&Type>) {
        if let Some(name) = name {
            self.set_property("name", name)
        }
        if let Some(enum_type) = enum_type {
            self.set_property("type", enum_type.to_string());
            match enum_type {
                Type::CirclePokemon | Type::CircleSmartPokemon => {
                    self.set_property("radius", 70);
                }
                Type::CircleRaid | Type::CircleSmartRaid => {
                    self.set_property("radius", 700);
                }
                Type::ManualQuest => {
                    self.set_property("radius", 80);
                }
                _ => {}
            }
        }
    }
}

pub fn sql_raw(area: &FeatureCollection) -> String {
    let mut string = "".to_string();
    for (i, feature) in area.features.iter().enumerate() {
        string = format!(
            "{} {} ST_CONTAINS(ST_GeomFromGeoJSON('{}', 2, 0), POINT(lon, lat))",
            string,
            if i == 0 { "WHERE" } else { "OR" },
            feature.to_string()
        );
    }
    string
}

pub fn get_return_type(
    return_type: Option<String>,
    default_return_type: ReturnTypeArg,
) -> ReturnTypeArg {
    if let Some(return_type) = return_type {
        match return_type.to_lowercase().as_str() {
            "alttext" | "alt_text" => ReturnTypeArg::AltText,
            "text" => ReturnTypeArg::Text,
            "array" => match default_return_type {
                ReturnTypeArg::SingleArray => ReturnTypeArg::SingleArray,
                ReturnTypeArg::MultiArray => ReturnTypeArg::MultiArray,
                _ => ReturnTypeArg::SingleArray,
            },
            "singlearray" | "single_array" => ReturnTypeArg::SingleArray,
            "multiarray" | "multi_array" => ReturnTypeArg::MultiArray,
            "struct" => match default_return_type {
                ReturnTypeArg::SingleStruct => ReturnTypeArg::SingleStruct,
                ReturnTypeArg::MultiStruct => ReturnTypeArg::MultiStruct,
                _ => ReturnTypeArg::SingleStruct,
            },
            "singlestruct" | "single_struct" => ReturnTypeArg::SingleStruct,
            "multistruct" | "multi_struct" => ReturnTypeArg::MultiStruct,
            "feature" => ReturnTypeArg::Feature,
            "featurecollection" | "feature_collection" => ReturnTypeArg::FeatureCollection,
            _ => default_return_type,
        }
    } else {
        default_return_type
    }
}

pub fn parse_text(text: &str, name: Option<String>, enum_type: Option<&Type>) -> Feature {
    let mut parsed = if text.starts_with("{") {
        match serde_json::from_str::<InstanceParsing>(text) {
            Ok(result) => match result {
                InstanceParsing::Feature(feat) => feat,
                InstanceParsing::Rdm(json) => {
                    let mut feature = match json.area {
                        RdmInstanceArea::Leveling(point) => {
                            convert::feature::from_single_point(point)
                        }
                        RdmInstanceArea::Single(area) => {
                            convert::feature::from_single_struct(area, enum_type)
                        }
                        RdmInstanceArea::Multi(area) => {
                            convert::feature::from_multi_struct(area, enum_type)
                        }
                    };
                    if let Some(radius) = json.radius {
                        feature.set_property("radius", radius);
                    }
                    feature
                }
            },
            Err(err) => {
                println!(
                    "Error Parsing Instance: {}\n{}",
                    name.clone().unwrap_or("".to_string()),
                    err
                );
                Feature::default()
            }
        }
    } else {
        convert::feature::from_text(text, enum_type)
    };
    parsed.add_instance_properties(name, enum_type);
    parsed
}

pub fn ensure_first_last<T>(points: Vec<[T; 2]>) -> Vec<[T; 2]>
where
    T: Float,
{
    if points.is_empty() {
        return points;
    }
    let mut points = points;
    if points[0] != points[points.len() - 1] {
        points.push(points[0]);
    }
    points
}

pub fn text_test(string: &str) -> bool {
    let split: Vec<&str> = string.split_whitespace().collect();
    match split[0].parse::<f64>() {
        Ok(_) => true,
        Err(_) => false,
    }
}
