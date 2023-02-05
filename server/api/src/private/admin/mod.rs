use super::*;

use serde::Deserialize;

pub mod geofence;
pub mod geofence_project;
pub mod project;
pub mod property;
pub mod route;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminReq {
    pub page: Option<usize>,
    pub per_page: Option<usize>,
    pub sort_by: Option<String>,
    pub order: Option<String>,
    pub q: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Search {
    pub query: String,
}

pub struct AdminReqParsed {
    pub page: usize,
    pub per_page: usize,
    pub sort_by: String,
    pub order: String,
    pub q: String,
}

impl AdminReq {
    fn parse(self) -> AdminReqParsed {
        AdminReqParsed {
            page: self.page.unwrap_or(0),
            order: self.order.unwrap_or("ASC".to_string()),
            per_page: self.per_page.unwrap_or(25),
            sort_by: self.sort_by.unwrap_or("id".to_string()),
            q: self.q.unwrap_or("".to_string()),
        }
    }
}
