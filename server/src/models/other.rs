#![allow(non_snake_case)]
use super::*;

#[derive(Debug, Serialize, Deserialize, Queryable)]
pub struct PixiMarker {
    pub id: String,
    pub iconId: String,
    pub position: (f64, f64),
}
