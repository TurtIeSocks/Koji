use num_traits::Float;

use crate::entities::sea_orm_active_enums::Type;

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

#[derive(Debug, Serialize, Deserialize)]
pub struct SinglePolygonData<T = f64>
where
    T: Float,
{
    pub area: Vec<LatLon<T>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MultiPolygonData<T = f64>
where
    T: Float,
{
    pub area: Vec<Vec<LatLon<T>>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GenericInstance<T = f32>
where
    T: Float,
{
    pub name: String,
    pub r#type: Type,
    pub data: Vec<Vec<[T; 2]>>,
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
