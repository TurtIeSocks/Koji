use geojson::{Feature, FeatureCollection, Geometry};
use serde::{Deserialize, Serialize};

use koji_core::{
    EnsurePoints, FenceType, MultiStruct, MultiVec, Poracle, SingleStruct, SingleVec, ToCollection,
    ToFeature,
};

pub mod args;
pub mod text;

/// Tagged union of every accepted geometry input shape. Stays in `model` for
/// now because the `Bound` variant references `args::BoundsArg`; it moves to
/// koji-core once args is broken up (P1d).
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum GeoFormats {
    Text(String),
    SingleArray(SingleVec),
    MultiArray(MultiVec),
    SingleStruct(SingleStruct),
    MultiStruct(MultiStruct),
    Geometry(Geometry),
    GeometryVec(Vec<Geometry>),
    Feature(Feature),
    FeatureVec(Vec<Feature>),
    FeatureCollection(FeatureCollection),
    Poracle(Vec<Poracle>),
    PoracleSingle(Poracle),
    Bound(args::BoundsArg),
}

impl ToCollection for GeoFormats {
    fn to_collection(self, name: Option<String>, enum_type: Option<FenceType>) -> FeatureCollection {
        match self {
            GeoFormats::Text(area) => area.to_collection(name, enum_type),
            GeoFormats::SingleArray(area) => area.to_collection(name, enum_type),
            GeoFormats::MultiArray(area) => area.to_collection(name, enum_type),
            GeoFormats::SingleStruct(area) => area.to_collection(name, enum_type),
            GeoFormats::MultiStruct(area) => area.to_collection(name, enum_type),
            GeoFormats::Geometry(area) => area.to_feature(enum_type).to_collection(name, None),
            GeoFormats::GeometryVec(area) => area.to_collection(name, enum_type),
            GeoFormats::Feature(area) => area.to_collection(name, enum_type),
            GeoFormats::FeatureVec(area) => area.to_collection(name, enum_type),
            GeoFormats::FeatureCollection(area) => area.to_collection(name, enum_type),
            GeoFormats::Poracle(area) => area.to_collection(name, enum_type),
            GeoFormats::PoracleSingle(area) => vec![area].to_collection(name, enum_type),
            GeoFormats::Bound(area) => vec![
                [area.min_lat, area.min_lon],
                [area.min_lat, area.max_lon],
                [area.max_lat, area.max_lon],
                [area.max_lat, area.min_lon],
                [area.min_lat, area.min_lon],
            ]
            .to_collection(name, enum_type),
        }
        .ensure_first_last()
    }
}
