mod collection;
mod feature;
#[allow(clippy::module_inception)]
mod geometry;
mod koji_geojson;
mod koji_geometry;
mod koji_meta;
mod multi_struct;
mod multi_vec;
mod point_array;
mod point_struct;
mod poracle;
mod single_struct;
mod single_vec;
mod text;

pub use koji_geojson::KojiGeojsonError;
pub use koji_geometry::{KojiGeometry, KojiGeometryCollection};
pub use koji_meta::KojiMeta;
pub use multi_struct::MultiStruct;
pub use multi_vec::MultiVec;
pub use point_array::PointArray;
pub use point_struct::PointStruct;
pub use poracle::Poracle;
pub use single_struct::SingleStruct;
pub use single_vec::SingleVec;
pub use text::text_test;

use geo::Point;
use geojson::{Bbox, Feature, FeatureCollection, Geometry, Value};
use serde::{Deserialize, Serialize};

use crate::{FeatureCtx, FenceType};

pub type Precision = f64;

/// Generates the conversions that are identical for every single-feature type,
/// derived from its hand-written `to_single_vec` / `to_single_struct` /
/// `to_feature`.
///
/// - `wrapper_conversions!(T)` emits all four wrappers, including the standard
///   single-feature `to_collection` (one feature, bbox copied from it).
/// - `wrapper_conversions!(@wrappers T)` emits only the three that never vary,
///   leaving `ToCollection` to a hand-written impl — used by `SingleVec`, whose
///   `to_collection` has an emptiness / `len > 1` guard the others lack.
macro_rules! wrapper_conversions {
    (@wrappers $t:ty) => {
        impl ToMultiVec for $t {
            fn to_multi_vec(self) -> MultiVec {
                vec![self.to_single_vec()]
            }
        }
        impl ToMultiStruct for $t {
            fn to_multi_struct(self) -> MultiStruct {
                vec![self.to_single_struct()]
            }
        }
        impl ToPoracle for $t {
            fn to_poracle(self) -> Poracle {
                Poracle {
                    path: Some(self.to_single_vec()),
                    ..Default::default()
                }
            }
        }
    };
    ($t:ty) => {
        wrapper_conversions!(@wrappers $t);
        impl ToCollection for $t {
            fn to_collection(self, ctx: &FeatureCtx) -> FeatureCollection {
                let feature = self.to_feature(ctx);
                FeatureCollection {
                    bbox: feature.bbox.clone(),
                    features: vec![feature],
                    foreign_members: None,
                }
            }
        }
    };
}
pub(crate) use wrapper_conversions;

pub trait EnsurePoints {
    fn ensure_first_last(self) -> Self;
}

pub trait EnsureProperties {
    fn ensure_properties(self, ctx: &FeatureCtx) -> Self;
}

/// [min_lon, min_lat, max_lon, max_lat]
pub trait GetBbox {
    fn get_bbox(&self) -> Option<Bbox>;
}

pub trait ValueHelpers {
    fn get_geojson_value(self, enum_type: FenceType) -> Value;
    fn point(self) -> Value;
    fn multi_point(self) -> Value;
    fn polygon(self) -> Value;
    fn multi_polygon(self) -> Value;
}

pub trait GeometryHelpers {
    fn simplify(self) -> Self;
}

pub trait FeatureHelpers {
    fn add_instance_properties(&mut self, ctx: &FeatureCtx);
    fn remove_last_coord(self) -> Self;
    fn remove_internal_props(self) -> Self;
}

pub trait ToPointArray {
    fn to_point_array(self) -> point_array::PointArray;
}

pub trait ToSingleVec {
    fn to_single_vec(self) -> single_vec::SingleVec;
}

pub trait ToMultiVec {
    fn to_multi_vec(self) -> multi_vec::MultiVec;
}

pub trait ToPointStruct {
    fn to_struct(self) -> point_struct::PointStruct;
}

pub trait ToSingleStruct {
    fn to_single_struct(self) -> single_struct::SingleStruct;
}

pub trait ToMultiStruct {
    fn to_multi_struct(self) -> multi_struct::MultiStruct;
}

pub trait ToFeature {
    fn to_feature(self, ctx: &FeatureCtx) -> Feature;
}

pub trait ToFeatureVec {
    fn to_feature_vec(self) -> Vec<Feature>;
}

pub trait ToCollection {
    fn to_collection(self, ctx: &FeatureCtx) -> FeatureCollection;
}

pub trait ToPoracle {
    fn to_poracle(self) -> poracle::Poracle;
}

pub trait ToPoracleVec {
    fn to_poracle_vec(self) -> Vec<poracle::Poracle>;
}

pub trait ToText {
    fn to_text(self, sep_1: &str, sep_2: &str, poly_sep: bool) -> String;
}

pub trait ToGeometry {
    fn to_geometry(self) -> Geometry;
}

pub trait ToGeometryVec {
    fn to_geometry_vec(self) -> Vec<Geometry>;
}

pub trait ToSql {
    fn to_sql(self) -> String;
}

#[derive(Debug, Clone)]
pub struct BBox {
    pub min_x: f64,
    pub min_y: f64,
    pub max_x: f64,
    pub max_y: f64,
}

impl Default for BBox {
    fn default() -> BBox {
        BBox {
            min_x: f64::INFINITY,
            min_y: f64::INFINITY,
            max_x: f64::NEG_INFINITY,
            max_y: f64::NEG_INFINITY,
        }
    }
}

impl BBox {
    pub fn new(points: &[Point]) -> BBox {
        let mut base = BBox {
            min_x: f64::INFINITY,
            min_y: f64::INFINITY,
            max_x: f64::NEG_INFINITY,
            max_y: f64::NEG_INFINITY,
        };
        for point in points.iter() {
            base.update(point);
        }
        base
    }
    pub fn update(&mut self, coord: &Point) {
        self.min_x = self.min_x.min(coord.x());
        self.min_y = self.min_y.min(coord.y());
        self.max_x = self.max_x.max(coord.x());
        self.max_y = self.max_y.max(coord.y());
    }
    pub fn get_poly(&self) -> Vec<Vec<Vec<f64>>> {
        vec![vec![
            vec![self.min_x, self.min_y],
            vec![self.min_x, self.max_y],
            vec![self.max_x, self.max_y],
            vec![self.max_x, self.min_y],
            vec![self.min_x, self.min_y],
        ]]
    }
    pub fn get_geojson_bbox(&self) -> Option<Bbox> {
        Some(vec![self.min_x, self.max_x, self.min_y, self.max_y])
    }
}
