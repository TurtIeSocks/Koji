use geojson::Feature;
use num_traits::Float;

use super::*;

#[derive(Debug, Serialize, Deserialize, Clone, FromQueryResult)]
pub struct LatLon<T = f64>
where
    T: Float,
{
    pub lat: T,
    pub lon: T,
}

#[derive(Debug, Serialize, Deserialize, Clone, FromQueryResult)]
pub struct TrimmedSpawn<T = f64>
where
    T: Float,
{
    pub lat: T,
    pub lon: T,
    pub despawn_sec: Option<u16>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum RdmInstanceArea {
    Leveling(LatLon),
    Single(Vec<LatLon>),
    Multi(Vec<Vec<LatLon>>),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RdmInstance {
    pub area: RdmInstanceArea,
    pub timezone_offset: Option<i32>,
    pub is_event: Option<bool>,
    pub min_level: Option<u8>,
    pub max_level: Option<u8>,
    pub pokemon_ids: Option<Vec<String>>,
    pub scatter_pokemon_ids: Option<Vec<u16>>,
    pub delay_logout: Option<u16>,
    pub quest_mode: Option<String>,
    pub spin_limit: Option<u16>,
    pub radius: Option<u32>,
    pub store_data: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GenericData<T = f64>
where
    T: Float,
{
    pub i: String,
    pub p: [T; 2],
}

impl<T> GenericData<T>
where
    T: Float,
{
    pub fn new(i: String, lat: T, lon: T) -> Self {
        GenericData { i, p: [lat, lon] }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum InstanceParsing {
    // Text(String),
    Feature(Feature),
    Rdm(RdmInstance),
}
