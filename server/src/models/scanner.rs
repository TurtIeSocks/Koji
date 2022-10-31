use super::*;

#[derive(Debug, Serialize, Deserialize, Clone, FromQueryResult)]
pub struct LatLon {
    pub lat: f64,
    pub lon: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InstanceData {
    pub area: Vec<Vec<LatLon>>,
    pub delay_logout: u32,
    pub is_event: bool,
    pub max_level: u8,
    pub min_level: u8,
    pub quest_mode: String,
    pub spin_limit: u16,
    pub timezone_offset: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GenericData {
    pub i: String,
    pub p: [f64; 2],
}

impl GenericData {
    pub fn new(i: String, lat: f64, lon: f64) -> Self {
        GenericData { i, p: [lat, lon] }
    }
}
