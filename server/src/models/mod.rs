use super::*;

use num_traits::Float;
use sea_orm::{DatabaseConnection, FromQueryResult};
use serde::{Deserialize, Serialize};

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
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CustomError {
    pub message: String,
}
