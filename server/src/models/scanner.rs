use super::*;

#[derive(Debug, Serialize, Deserialize, Clone, FromQueryResult)]
pub struct Spawnpoint<T = f64>
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
    Leveling(PointStruct),
    Single(SingleStruct),
    Multi(MultiStruct),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RdmInstance {
    pub area: RdmInstanceArea,
    pub radius: Option<u32>,
    // pub timezone_offset: Option<i32>,
    // pub is_event: Option<bool>,
    // pub min_level: Option<u8>,
    // pub max_level: Option<u8>,
    // pub delay_logout: Option<u16>,
    // pub quest_mode: Option<String>,
    // pub spin_limit: Option<u16>,
    // pub store_data: Option<bool>,
    // pub iv_queue_limit: Option<i32>,
    // pub account_group: Option<String>,
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
