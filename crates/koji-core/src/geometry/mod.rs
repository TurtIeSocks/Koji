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
mod text;

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
pub use text::text_test;

use geo::Point;
use geojson::Bbox;

pub type Precision = f64;

pub trait EnsurePoints {
    fn ensure_first_last(self) -> Self;
}

/// [min_lon, min_lat, max_lon, max_lat]
pub trait GetBbox {
    fn get_bbox(&self) -> Option<Bbox>;
}

pub trait GeometryHelpers {
    fn simplify(self) -> Self;
}

pub trait FeatureHelpers {
    fn remove_internal_props(self) -> Self;
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
