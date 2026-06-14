use serde::{Deserialize, Serialize};

use super::*;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PointStruct {
    pub lat: Precision,
    pub lon: Precision,
}
impl Default for PointStruct {
    fn default() -> PointStruct {
        PointStruct { lat: 0., lon: 0. }
    }
}

impl From<PointArray> for PointStruct {
    fn from(p: PointArray) -> Self {
        PointStruct {
            lat: p[0],
            lon: p[1],
        }
    }
}
