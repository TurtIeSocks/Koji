use geojson::{Feature, FeatureCollection};

use super::*;
use crate::models::scanner::LatLon;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub start_lat: f64,
    pub start_lon: f64,
    pub tile_server: String,
    pub scanner_type: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MapBounds {
    pub min_lat: f64,
    pub min_lon: f64,
    pub max_lat: f64,
    pub max_lon: f64,
}

pub enum ArrayType {
    S(Vec<[f64; 2]>),
    M(Vec<Vec<[f64; 2]>>),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum AreaInput {
    Text(String),
    SingleArray(Vec<[f64; 2]>),
    MultiArray(Vec<Vec<[f64; 2]>>),
    SingleStruct(Vec<LatLon>),
    MultiStruct(Vec<Vec<LatLon>>),
    Feature(Feature),
    FeatureCollection(FeatureCollection),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ReturnType {
    AltText,
    Text,
    SingleArray,
    MultiArray,
    SingleStruct,
    MultiStruct,
    Feature,
    FeatureCollection,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RouteGeneration {
    pub instance: Option<String>,
    pub radius: Option<f64>,
    pub min_points: Option<usize>,
    pub generations: Option<usize>,
    pub devices: Option<usize>,
    pub data_points: Option<Vec<LatLon>>,
    pub area: Option<AreaInput>,
    pub fast: Option<bool>,
    pub return_type: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CustomError {
    pub message: String,
}
