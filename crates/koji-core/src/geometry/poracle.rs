use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

use super::*;
use crate::UnknownId;

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Poracle {
    pub id: Option<UnknownId>,
    pub name: Option<String>,
    pub color: Option<String>,
    pub group: Option<String>,
    pub description: Option<String>,
    pub user_selectable: Option<bool>,
    pub display_in_matches: Option<bool>,
    pub path: Option<single_vec::SingleVec>,
    pub multipath: Option<multi_vec::MultiVec>,
}

impl Default for Poracle {
    fn default() -> Poracle {
        Poracle {
            id: Some(UnknownId::Number(0)),
            name: Some("".to_string()),
            color: None,
            group: None,
            description: None,
            user_selectable: None,
            display_in_matches: None,
            path: Some(vec![]),
            multipath: None,
        }
    }
}
