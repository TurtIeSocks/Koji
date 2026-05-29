use sea_orm::FromQueryResult;
use serde::{Deserialize, Serialize};

/// Query-row for `SELECT lat, lon` from scanner tables. Converts into the pure
/// `koji_core::PointStruct`.
#[derive(Debug, FromQueryResult)]
pub struct LatLonRow {
    pub lat: f64,
    pub lon: f64,
}

impl From<LatLonRow> for koji_core::PointStruct {
    fn from(r: LatLonRow) -> Self {
        koji_core::PointStruct {
            lat: r.lat,
            lon: r.lon,
        }
    }
}

#[derive(Debug, Clone, FromQueryResult)]
pub struct Spawnpoint {
    pub lat: f64,
    pub lon: f64,
    pub despawn_sec: Option<u16>,
}

#[derive(Debug, FromQueryResult, Serialize, Deserialize, Clone)]
pub struct Total {
    pub total: i32,
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

impl koji_core::ToPointArray for GenericData {
    fn to_point_array(self) -> koji_core::PointArray {
        self.p
    }
}
impl koji_core::ToPointStruct for GenericData {
    fn to_struct(self) -> koji_core::PointStruct {
        koji_core::PointStruct {
            lat: self.p[0],
            lon: self.p[1],
        }
    }
}

pub trait GenericDataToVec {
    fn to_single_vec(self) -> koji_core::SingleVec;
}

impl GenericDataToVec for Vec<GenericData> {
    fn to_single_vec(self) -> koji_core::SingleVec {
        self.into_iter()
            .map(|p| koji_core::ToPointArray::to_point_array(p))
            .collect()
    }
}
