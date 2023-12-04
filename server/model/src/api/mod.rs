use super::{
    db::{sea_orm_active_enums::Type, InstanceParsing, RdmInstanceArea},
    *,
};

use geo::Point;
use geojson::{Bbox, Geometry, Value};
use sea_orm::FromQueryResult;

pub mod args;
pub mod cluster_mode;
pub mod collection;
pub mod feature;
pub mod geometry;
pub mod multi_struct;
pub mod multi_vec;
pub mod point_array;
pub mod point_struct;
pub mod poracle;
pub mod single_struct;
pub mod single_vec;
pub mod sort_by;
pub mod text;

pub type Precision = f64;

pub trait EnsurePoints {
    fn ensure_first_last(self) -> Self;
}

pub trait EnsureProperties {
    fn ensure_properties(self, name: Option<String>, enum_type: Option<Type>) -> Self;
}

/// [min_lon, min_lat, max_lon, max_lat]
pub trait GetBbox {
    fn get_bbox(&self) -> Option<Bbox>;
}

pub trait ValueHelpers {
    fn get_geojson_value(self, enum_type: Type) -> Value;
    fn point(self) -> Value;
    fn multi_point(self) -> Value;
    fn polygon(self) -> Value;
    fn multi_polygon(self) -> Value;
}

pub trait GeometryHelpers {
    fn simplify(self) -> Self;
}

pub trait FeatureHelpers {
    fn add_instance_properties(&mut self, name: Option<String>, enum_type: Option<Type>);
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
    fn to_feature(self, enum_type: Option<Type>) -> Feature;
}

pub trait ToFeatureVec {
    fn to_feature_vec(self) -> Vec<Feature>;
}

pub trait ToCollection {
    fn to_collection(self, name: Option<String>, enum_type: Option<Type>) -> FeatureCollection;
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

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum GeoFormats {
    Text(String),
    SingleArray(single_vec::SingleVec),
    MultiArray(multi_vec::MultiVec),
    SingleStruct(single_struct::SingleStruct),
    MultiStruct(multi_struct::MultiStruct),
    Geometry(Geometry),
    GeometryVec(Vec<Geometry>),
    Feature(Feature),
    FeatureVec(Vec<Feature>),
    FeatureCollection(FeatureCollection),
    Poracle(Vec<poracle::Poracle>),
    PoracleSingle(poracle::Poracle),
    Bound(args::BoundsArg),
}

impl ToCollection for GeoFormats {
    fn to_collection(self, name: Option<String>, enum_type: Option<Type>) -> FeatureCollection {
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
    pub fn new(points: &Vec<Point>) -> BBox {
        let mut base = BBox {
            min_x: f64::INFINITY,
            min_y: f64::INFINITY,
            max_x: f64::NEG_INFINITY,
            max_y: f64::NEG_INFINITY,
        };
        for point in points.into_iter() {
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
        // println!(
        //     "{}, {}\n{}, {}\n{}, {}\n{}, {}\n{}, {}\n",
        //     self.min_y,
        //     self.min_x,
        //     self.max_y,
        //     self.min_x,
        //     self.max_y,
        //     self.max_x,
        //     self.min_y,
        //     self.max_x,
        //     self.min_y,
        //     self.min_x,
        // )
    }
    pub fn get_geojson_bbox(&self) -> Option<Bbox> {
        Some(vec![self.min_x, self.max_x, self.min_y, self.max_y])
    }
    // pub fn get_center(&self) -> Coord {
    //     Coord {
    //         x: (self.min_x + self.max_x) / 2.0,
    //         y: (self.min_y + self.max_y) / 2.0,
    //     }
    // }
}
