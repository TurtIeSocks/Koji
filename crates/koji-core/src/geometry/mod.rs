mod collection;
mod feature;
#[allow(clippy::module_inception)]
mod geometry;
mod koji_bbox;
mod koji_geojson;
mod koji_geometry;
mod koji_meta;
mod koji_output;
mod multi_struct;
mod multi_vec;
mod point_array;
mod point_struct;
mod poracle;
mod single_struct;
mod single_vec;

pub use koji_bbox::KojiBbox;
pub use koji_geojson::KojiGeojsonError;
pub use koji_geometry::{KojiGeometry, KojiGeometryCollection};
pub use koji_meta::KojiMeta;
pub use koji_output::{
    single_vec_to_multipoint, single_vec_to_multipoint_feature, single_vec_to_polygon,
    single_vec_to_polygon_feature,
};
pub use multi_struct::MultiStruct;
pub use multi_vec::MultiVec;
pub use point_array::PointArray;
pub use point_struct::PointStruct;
pub use poracle::Poracle;
pub use single_struct::SingleStruct;
pub use single_vec::SingleVec;

pub type Precision = f64;

pub trait EnsurePoints {
    fn ensure_first_last(self) -> Self;
}

pub trait GeometryHelpers {
    fn simplify(self) -> Self;
}

pub trait FeatureHelpers {
    fn remove_internal_props(self) -> Self;
}
