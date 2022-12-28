use super::*;

use serde::Deserialize;

pub mod geofence;
pub mod project;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminReq {
    pub page: usize,
    pub per_page: usize,
    pub sort_by: String,
    pub order: String,
}
