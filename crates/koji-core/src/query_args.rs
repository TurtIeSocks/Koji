use serde::{Deserialize, Serialize};

use crate::Precision;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiQueryArgs {
    /// If true, internal database properties are added with a `__` prefix
    /// Adds a generated `id` property
    /// Not encouraged to use outside of development
    pub internal: Option<bool>,

    // -------------------------------------------------------------------------
    // Adds the respective property to the return Feature/FeatureCollection
    // It is encouraged to add these properties through the admin panel instead of using these args!
    /// If true, the `id` property is added
    pub id: Option<bool>,
    /// If true, the `name` property is added
    pub name: Option<bool>,
    /// If true, the `mode` property is added
    pub mode: Option<bool>,
    /// If true, the `geofence_id` property is added
    pub geofence_id: Option<bool>,
    /// If true, the `parent` property is added
    pub parent: Option<bool>,

    // -------------------------------------------------------------------------
    // Extras
    /// custom return type of the API request
    ///
    /// Options: [ReturnTypeArg]
    pub rt: Option<String>,
    /// If true, the `group` property is set from the parent property
    pub group: Option<bool>,
    /// If true, the full 64 bit coordinates are used
    pub fullcoords: Option<bool>,

    // -------------------------------------------------------------------------
    // Hierarchy (recursive geofence subtree). Mutually exclusive — supplying
    // both is an HTTP 400.
    /// Cumulative subtree: the anchor (depth 0) through `depth` levels of
    /// descendants, inclusive. `depth=0` returns the anchor(s) only.
    pub depth: Option<u32>,
    /// Exactly the geofences `level` levels below the anchor. `level=0` returns
    /// the anchor(s) themselves.
    pub level: Option<u32>,

    // -------------------------------------------------------------------------
    // Name Property Manipulation
    /// If true, the entire `name` property is set to lowercase
    pub lowercase: Option<bool>,
    /// If true, the entire `name` property is set to uppercase
    pub uppercase: Option<bool>,
    /// If provided, the `name` property is split at the provided string/character, each word is capitalized, then rejoined with the same character
    pub capitalize: Option<String>,
    /// If true, the first character of the `name` property is capitalized
    pub capfirst: Option<bool>,
    /// If true, the `parent` property is added as a prefix to the `name`, separated by the provided string/character
    pub parentstart: Option<String>,
    /// If true, the `parent` property is added as a suffix to the `name`, separated by the provided string/character
    pub parentend: Option<String>,
    /// If the `name` property has the `parent` name as part of its value, the `parent` name is replaced with the given string/character
    pub parentreplace: Option<String>,
    /// Spaces in the `name` property are replaced with the given string/character
    pub space: Option<String>,
    /// Underscores in the `name` property are replaced with the given string/character
    pub underscore: Option<String>,
    /// Dashes/Hyphens in the `name` property are replaced with the given string/character
    pub dash: Option<String>,
    /// Replaces any provided string/character with `""` (empty string)
    pub replace: Option<String>,
    /// Trims x number of characters from the front of the `name` property
    pub trimstart: Option<usize>,
    /// Trims x number of characters from the back of the `name` property
    pub trimend: Option<usize>,
    /// If true, the polish characters are converted to ascii
    pub unpolish: Option<bool>,
    /// If true, the manual parent property will be ignored
    pub ignoremanualparent: Option<bool>,
    /// If true, all non-alphanumeric characters are removed from the `name` property
    /// (excludes spaces, dashes, and underscores)
    pub alphanumeric: Option<bool>,
    /// Exclude areas with the specified properties, no matter the value or type.
    ///
    /// Property keys separated by a comma
    pub excludeproperties: Option<String>,
    /// Excludes areas with the specified parent names
    ///
    /// A list of parents names separated by a comma that are to be excluded by the API query
    pub excludeparents: Option<String>,
    /// A list of names separated by a comma that are to be excluded by the API query
    pub exclude: Option<String>,
}

impl Default for ApiQueryArgs {
    fn default() -> Self {
        Self {
            internal: Some(true),
            id: None,
            name: None,
            mode: None,
            geofence_id: None,
            parent: None,
            rt: None,
            fullcoords: None,
            depth: None,
            level: None,
            lowercase: None,
            uppercase: None,
            capitalize: None,
            capfirst: None,
            parentstart: None,
            parentend: None,
            parentreplace: None,
            space: None,
            underscore: None,
            dash: None,
            replace: None,
            group: None,
            trimstart: None,
            trimend: None,
            unpolish: None,
            ignoremanualparent: None,
            alphanumeric: None,
            excludeproperties: None,
            excludeparents: None,
            exclude: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoundsArg {
    pub min_lat: Precision,
    pub min_lon: Precision,
    pub max_lat: Precision,
    pub max_lon: Precision,
    pub last_seen: Option<u32>,
    pub ids: Option<Vec<String>>,
    pub tth: Option<SpawnpointTth>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum SpawnpointTth {
    All,
    Known,
    Unknown,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminReq {
    pub page: Option<u64>,
    pub per_page: Option<u64>,
    pub sort_by: Option<String>,
    pub order: Option<String>,
    pub q: Option<String>,
    pub geotype: Option<String>,
    pub project: Option<u32>,
    pub mode: Option<String>,
    pub parent: Option<u32>,
    pub geofenceid: Option<u32>,
    pub pointsmin: Option<u32>,
    pub pointsmax: Option<u32>,
}

impl AdminReq {
    pub fn parse(self) -> AdminReqParsed {
        AdminReqParsed {
            page: self.page.unwrap_or(0),
            order: self.order.unwrap_or("ASC".to_string()),
            per_page: self.per_page.unwrap_or(25),
            sort_by: self.sort_by.unwrap_or("id".to_string()),
            q: self.q.unwrap_or("".to_string()),
            geotype: self.geotype,
            project: self.project,
            mode: self.mode,
            parent: self.parent,
            geofenceid: self.geofenceid,
            pointsmin: self.pointsmin,
            pointsmax: self.pointsmax,
        }
    }
}

#[derive(Debug)]
pub struct AdminReqParsed {
    pub page: u64,
    pub per_page: u64,
    pub sort_by: String,
    pub order: String,
    pub q: String,
    pub geotype: Option<String>,
    pub project: Option<u32>,
    pub mode: Option<String>,
    pub parent: Option<u32>,
    pub geofenceid: Option<u32>,
    pub pointsmin: Option<u32>,
    pub pointsmax: Option<u32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserializes_depth() {
        let args: ApiQueryArgs = serde_json::from_value(serde_json::json!({ "depth": 2 })).unwrap();
        assert_eq!(args.depth, Some(2));
        assert_eq!(args.level, None);
    }

    #[test]
    fn deserializes_level() {
        let args: ApiQueryArgs = serde_json::from_value(serde_json::json!({ "level": 3 })).unwrap();
        assert_eq!(args.level, Some(3));
        assert_eq!(args.depth, None);
    }

    #[test]
    fn depth_and_level_default_none() {
        let args: ApiQueryArgs = serde_json::from_value(serde_json::json!({})).unwrap();
        assert_eq!(args.depth, None);
        assert_eq!(args.level, None);
    }
}
