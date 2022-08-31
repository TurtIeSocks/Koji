use super::*;
use crate::models::scanner::GenericData;

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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RouteGeneration {
    pub instance: Option<String>,
    pub radius: Option<f64>,
    pub min_points: Option<i32>,
    pub generations: Option<usize>,
    pub devices: Option<usize>,
    pub data_points: Option<Vec<GenericData>>,
    pub area: Option<Vec<[f64; 2]>>,
    pub fast: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CustomError {
    pub message: String,
}
