use super::*;

#[derive(Debug, Serialize, Deserialize)]
pub struct BoundsArg {
    pub min_lat: f64,
    pub min_lon: f64,
    pub max_lat: f64,
    pub max_lon: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ReturnTypeArg {
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
pub enum DataPointsArg {
    Array(SingleVec),
    Struct(SingleStruct),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Args {
    pub instance: Option<String>,
    pub radius: Option<f64>,
    pub min_points: Option<usize>,
    pub generations: Option<usize>,
    pub devices: Option<usize>,
    pub data_points: Option<DataPointsArg>,
    pub area: Option<GeoFormats>,
    pub fast: Option<bool>,
    pub return_type: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConfigResponse {
    pub start_lat: f64,
    pub start_lon: f64,
    pub tile_server: String,
    pub scanner_type: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Response {
    message: Option<String>,
    status: Option<String>,
    status_code: Option<u16>,
    data: GeoFormats,
}
