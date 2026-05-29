use geojson::{Feature, FeatureCollection, Geometry};
use serde::{Deserialize, Serialize};

use koji_core::{
    EnsurePoints, FeatureCtx, MultiStruct, MultiVec, Poracle, SingleStruct, SingleVec,
    ToCollection, ToFeature,
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
    fn to_collection(self, ctx: &FeatureCtx) -> FeatureCollection {
        match self {
            GeoFormats::Text(area) => area.to_collection(ctx),
            GeoFormats::SingleArray(area) => area.to_collection(ctx),
            GeoFormats::MultiArray(area) => area.to_collection(ctx),
            GeoFormats::SingleStruct(area) => area.to_collection(ctx),
            GeoFormats::MultiStruct(area) => area.to_collection(ctx),
            GeoFormats::Geometry(area) => area.to_feature(ctx).to_collection(ctx),
            GeoFormats::GeometryVec(area) => area.to_collection(ctx),
            GeoFormats::Feature(area) => area.to_collection(ctx),
            GeoFormats::FeatureVec(area) => area.to_collection(ctx),
            GeoFormats::FeatureCollection(area) => area.to_collection(ctx),
            GeoFormats::Poracle(area) => area.to_collection(ctx),
            GeoFormats::PoracleSingle(area) => vec![area].to_collection(ctx),
            GeoFormats::Bound(area) => vec![
                [area.min_lat, area.min_lon],
                [area.min_lat, area.max_lon],
                [area.max_lat, area.max_lon],
                [area.max_lat, area.min_lon],
                [area.min_lat, area.min_lon],
            ]
            .to_collection(ctx),
        }
        .ensure_first_last()
    }
}
