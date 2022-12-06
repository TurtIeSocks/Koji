use super::*;

use geo::Coordinate;
use num_traits::Float;
use sea_orm::{DatabaseConnection, FromQueryResult};
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

pub mod api;
pub mod scanner;

#[derive(Clone)]
pub struct KojiDb {
    pub data_db: DatabaseConnection,
    pub unown_db: Option<DatabaseConnection>,
}

type Precision = f64;

pub type PointArray<T = Precision> = [T; 2];
pub type SingleVec<T = Precision> = Vec<PointArray<T>>;
pub type MultiVec<T = Precision> = Vec<Vec<PointArray<T>>>;

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Poracle {
    pub id: Option<u64>,
    pub name: Option<String>,
    pub color: Option<String>,
    pub group: Option<String>,
    pub description: Option<String>,
    pub user_selectable: Option<bool>,
    pub display_in_matches: Option<bool>,
    pub path: Option<SingleVec>,
    pub multipath: Option<MultiVec>,
}

#[derive(Debug, Serialize, Deserialize, Clone, FromQueryResult)]
pub struct PointStruct<T: Float = Precision> {
    pub lat: T,
    pub lon: T,
}
pub type SingleStruct<T = Precision> = Vec<PointStruct<T>>;
pub type MultiStruct<T = Precision> = Vec<Vec<PointStruct<T>>>;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ArrayType<T: Float = Precision> {
    S(SingleVec<T>),
    M(MultiVec<T>),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum GeoFormats {
    Text(String),
    SingleArray(SingleVec),
    MultiArray(MultiVec),
    SingleStruct(SingleStruct),
    MultiStruct(MultiStruct),
    Feature(Feature),
    FeatureVec(Vec<Feature>),
    FeatureCollection(FeatureCollection),
    Poracle(Vec<Poracle>),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CustomError {
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct BBox {
    pub min_x: f64,
    pub min_y: f64,
    pub max_x: f64,
    pub max_y: f64,
}

impl BBox {
    pub fn new(points: Option<&Vec<Coordinate>>) -> BBox {
        let mut base = BBox {
            min_x: f64::INFINITY,
            min_y: f64::INFINITY,
            max_x: f64::NEG_INFINITY,
            max_y: f64::NEG_INFINITY,
        };
        if let Some(points) = points {
            for point in points.into_iter() {
                base.min_x = base.min_x.min(point.x);
                base.min_y = base.min_y.min(point.y);
                base.max_x = base.max_x.max(point.x);
                base.max_y = base.max_y.max(point.y);
            }
        }
        base
    }
    pub fn update(&mut self, coord: Coordinate) {
        self.min_x = self.min_x.min(coord.x);
        self.min_y = self.min_y.min(coord.y);
        self.max_x = self.max_x.max(coord.x);
        self.max_y = self.max_y.max(coord.y);
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
    // pub fn get_center(&self) -> Coordinate {
    //     Coordinate {
    //         x: (self.min_x + self.max_x) / 2.0,
    //         y: (self.min_y + self.max_y) / 2.0,
    //     }
    // }
}
