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

pub trait GenericDataToVec {
    fn to_single_vec(self) -> koji_core::SingleVec;
}

impl GenericDataToVec for Vec<GenericData> {
    fn to_single_vec(self) -> koji_core::SingleVec {
        // `GenericData.p` is already `[lat, lon]` — the exact `PointArray` shape,
        // so map it through directly (no `To*` matrix trait).
        self.into_iter().map(|g| g.p).collect()
    }
}
