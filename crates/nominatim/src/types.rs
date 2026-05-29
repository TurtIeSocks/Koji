pub use serde::{Deserialize, Serialize};
pub use std::collections::HashMap;
use std::convert::Infallible;
use std::fmt;
use std::str::FromStr;

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
#[serde(from = "String")]
#[serde(into = "String")]
pub enum OsmType {
    Node,
    Way,
    Relation,
    Other(String),
}

impl FromStr for OsmType {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<OsmType, Self::Err> {
        Ok(match s {
            "node" => Self::Node,
            "way" => Self::Way,
            "relation" => Self::Relation,
            _ => Self::Other(s.to_string()),
        })
    }
}

impl fmt::Display for OsmType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Node => write!(f, "node"),
            Self::Way => write!(f, "way"),
            Self::Relation => write!(f, "relation"),
            Self::Other(s) => write!(f, "{}", s),
        }
    }
}

impl From<String> for OsmType {
    fn from(s: String) -> Self {
        match s.as_str() {
            "node" => Self::Node,
            "way" => Self::Way,
            "relation" => Self::Relation,
            _ => Self::Other(s),
        }
    }
}

impl From<OsmType> for String {
    fn from(osm_type: OsmType) -> Self {
        match osm_type {
            OsmType::Node => "node".to_string(),
            OsmType::Way => "way".to_string(),
            OsmType::Relation => "relation".to_string(),
            OsmType::Other(s) => s,
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
pub struct Country {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub country: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub country_code: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
pub struct Region {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state_district: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub county: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
pub struct Municipality {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub municiplality: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub city: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub town: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub village: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
pub struct CityDistrict {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub city_district: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub district: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub borough: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suburb: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subdivision: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
pub struct Hamlet {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hamlet: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub croft: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub isolated_dwelling: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
pub struct Neighbourhood {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub neighbourhood: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allotments: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quarter: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
pub struct CityBlock {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub city_block: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub residental: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub farm: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub farmyard: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub industrial: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commercial: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retail: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
pub struct House {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub house_number: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub house_name: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
pub struct Place {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emergency: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub historic: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub military: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub natural: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub landuse: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub place: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub railway: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manmade: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aerialway: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub boundary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amenity: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aeroway: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub club: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub leisure: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub office: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub moutainpass: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shop: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tourism: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bridge: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tunnel: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub waterway: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
pub struct Address {
    pub continent: Option<String>,
    #[serde(flatten)]
    pub country: Country,
    #[serde(flatten)]
    pub region: Region,
    #[serde(flatten)]
    pub municipality: Municipality,
    #[serde(flatten)]
    pub city_district: CityDistrict,
    #[serde(flatten)]
    pub hamlet: Hamlet,
    #[serde(flatten)]
    pub neighbourhood: Neighbourhood,
    #[serde(flatten)]
    pub city_block: CityBlock,
    pub road: Option<String>,
    #[serde(flatten)]
    pub house: House,
    #[serde(flatten)]
    pub place: Place,
    pub postcode: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
pub enum ID {
    #[serde(deserialize_with = "crate::serde_utils::deserialize_from_string")]
    #[serde(serialize_with = "crate::serde_utils::serialize_as_string")]
    String(String),
    Num(u64),
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
pub struct Response {
    /// Reference to the Nominatim internal database ID.
    pub place_id: Option<ID>,
    /// The type of this response. Likely a `node`, `way` or `relation`.
    pub osm_type: Option<OsmType>,
    /// Reference to the OSM object
    pub osm_id: Option<ID>,
    #[serde(
        deserialize_with = "crate::serde_utils::deserialize_from_string_opt"
    )]
    #[serde(serialize_with = "crate::serde_utils::serialize_as_string_opt")]
    /// Longitude of the centroid of the object
    pub lon: Option<f64>,
    #[serde(
        deserialize_with = "crate::serde_utils::deserialize_from_string_opt"
    )]
    #[serde(serialize_with = "crate::serde_utils::serialize_as_string_opt")]
    /// Latitude of the centroid of the object
    pub lat: Option<f64>,
    /// A license
    pub licence: Option<String>,
    /// Dictionary of address details.
    pub address: Option<Address>,
    /// Full comma-separated address
    pub display_name: Option<String>,
    /// Link to class icon (if available)
    pub icon: Option<String>,
    /// The main OSM tag
    pub class: Option<String>,
    /// The main OSM tag
    pub r#type: Option<String>,
    /// Computed importance rank
    pub importance: Option<f64>,
    /// Bounding box
    pub boundingbox: [String; 4],
}
