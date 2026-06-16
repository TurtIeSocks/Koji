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
            Self::Other(s) => write!(f, "{s}"),
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
    #[serde(deserialize_with = "crate::serde_utils::deserialize_from_string_opt")]
    #[serde(serialize_with = "crate::serde_utils::serialize_as_string_opt")]
    /// Longitude of the centroid of the object
    pub lon: Option<f64>,
    #[serde(deserialize_with = "crate::serde_utils::deserialize_from_string_opt")]
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

#[cfg(test)]
mod tests {
    use super::*;

    // ── OsmType ──────────────────────────────────────────────────────────────

    #[test]
    fn osm_type_from_str_known() {
        assert_eq!("node".parse::<OsmType>().unwrap(), OsmType::Node);
        assert_eq!("way".parse::<OsmType>().unwrap(), OsmType::Way);
        assert_eq!("relation".parse::<OsmType>().unwrap(), OsmType::Relation);
    }

    #[test]
    fn osm_type_from_str_other() {
        assert_eq!(
            "area".parse::<OsmType>().unwrap(),
            OsmType::Other("area".into())
        );
    }

    #[test]
    fn osm_type_display() {
        assert_eq!(OsmType::Node.to_string(), "node");
        assert_eq!(OsmType::Way.to_string(), "way");
        assert_eq!(OsmType::Relation.to_string(), "relation");
        assert_eq!(OsmType::Other("foo".into()).to_string(), "foo");
    }

    #[test]
    fn osm_type_serde_roundtrip() {
        for s in ["node", "way", "relation", "weird"] {
            let json = format!(r#""{s}""#);
            let parsed: OsmType = serde_json::from_str(&json).unwrap();
            let back = serde_json::to_string(&parsed).unwrap();
            assert_eq!(back, json, "roundtrip failed for {s}");
        }
    }

    // ── Address ──────────────────────────────────────────────────────────────

    fn minimal_address_json() -> &'static str {
        r#"{
          "continent": null,
          "country": "Germany",
          "country_code": "de",
          "region": null,
          "state": "Bavaria",
          "state_district": null,
          "county": null,
          "municiplality": null,
          "city": "Munich",
          "town": null,
          "village": null,
          "city_district": null,
          "district": null,
          "borough": null,
          "suburb": null,
          "subdivision": null,
          "hamlet": null,
          "croft": null,
          "isolated_dwelling": null,
          "neighbourhood": null,
          "allotments": null,
          "quarter": null,
          "city_block": null,
          "residental": null,
          "farm": null,
          "farmyard": null,
          "industrial": null,
          "commercial": null,
          "retail": null,
          "road": "Marienplatz",
          "house_number": null,
          "house_name": null,
          "emergency": null,
          "historic": null,
          "military": null,
          "natural": null,
          "landuse": null,
          "place": null,
          "railway": null,
          "manmade": null,
          "aerialway": null,
          "boundary": null,
          "amenity": null,
          "aeroway": null,
          "club": null,
          "leisure": null,
          "office": null,
          "moutainpass": null,
          "shop": null,
          "tourism": null,
          "bridge": null,
          "tunnel": null,
          "waterway": null,
          "postcode": "80331"
        }"#
    }

    #[test]
    fn address_deserialize_fields() {
        let addr: Address = serde_json::from_str(minimal_address_json()).unwrap();
        assert_eq!(addr.country.country.as_deref(), Some("Germany"));
        assert_eq!(addr.country.country_code.as_deref(), Some("de"));
        assert_eq!(addr.region.state.as_deref(), Some("Bavaria"));
        assert_eq!(addr.municipality.city.as_deref(), Some("Munich"));
        assert_eq!(addr.road.as_deref(), Some("Marienplatz"));
        assert_eq!(addr.postcode.as_deref(), Some("80331"));
    }

    #[test]
    fn address_serde_roundtrip() {
        let addr: Address = serde_json::from_str(minimal_address_json()).unwrap();
        let json = serde_json::to_string(&addr).unwrap();
        let addr2: Address = serde_json::from_str(&json).unwrap();
        assert_eq!(addr, addr2);
    }

    // ── ID ───────────────────────────────────────────────────────────────────

    #[test]
    fn id_numeric_deserialized_as_num() {
        let id: ID = serde_json::from_str("12345").unwrap();
        assert_eq!(id, ID::Num(12345));
    }

    #[test]
    fn id_string_deserialized_as_string() {
        let id: ID = serde_json::from_str(r#""98765""#).unwrap();
        assert_eq!(id, ID::String("98765".into()));
    }

    #[test]
    fn id_string_serialized_back_to_string() {
        let id = ID::String("42".into());
        let s = serde_json::to_string(&id).unwrap();
        assert_eq!(s, r#""42""#);
    }

    // ── Response ─────────────────────────────────────────────────────────────

    const RESPONSE_JSON: &str = r#"{
      "place_id": 282563964,
      "licence": "Data © OpenStreetMap contributors, ODbL 1.0. https://osm.org/copyright",
      "osm_type": "relation",
      "osm_id": "62422",
      "boundingbox": ["48.0617474","48.2480911","11.3607562","11.7229579"],
      "lat": "48.1371079",
      "lon": "11.5753822",
      "display_name": "Munich, Bavaria, Germany",
      "class": "boundary",
      "type": "administrative",
      "importance": 0.7654
    }"#;

    #[test]
    fn response_deserialize_basic() {
        let r: Response = serde_json::from_str(RESPONSE_JSON).unwrap();
        assert_eq!(r.place_id, Some(ID::Num(282_563_964)));
        assert_eq!(r.osm_type, Some(OsmType::Relation));
        assert_eq!(r.osm_id, Some(ID::String("62422".into())));
        assert!((r.lat.unwrap() - 48.137_107_9).abs() < 1e-6);
        assert!((r.lon.unwrap() - 11.575_382_2).abs() < 1e-6);
        assert_eq!(r.display_name.as_deref(), Some("Munich, Bavaria, Germany"));
        assert_eq!(r.class.as_deref(), Some("boundary"));
        assert_eq!(r.r#type.as_deref(), Some("administrative"));
        assert!((r.importance.unwrap() - 0.7654).abs() < 1e-10);
        assert_eq!(r.boundingbox[0], "48.0617474");
        assert_eq!(r.boundingbox[3], "11.7229579");
    }

    #[test]
    fn response_lat_lon_none() {
        let json = r#"{
          "place_id": 1,
          "osm_type": "node",
          "osm_id": 1,
          "lat": null,
          "lon": null,
          "boundingbox": ["0","0","0","0"]
        }"#;
        let r: Response = serde_json::from_str(json).unwrap();
        assert!(r.lat.is_none());
        assert!(r.lon.is_none());
    }

    #[test]
    fn response_serde_roundtrip() {
        let r: Response = serde_json::from_str(RESPONSE_JSON).unwrap();
        let json = serde_json::to_string(&r).unwrap();
        let r2: Response = serde_json::from_str(&json).unwrap();
        assert_eq!(r.place_id, r2.place_id);
        assert_eq!(r.osm_type, r2.osm_type);
        // lat/lon round-trip through Display (f64 → string → f64): tolerance check
        let lat_diff = (r.lat.unwrap() - r2.lat.unwrap()).abs();
        assert!(lat_diff < 1e-6, "lat drift: {lat_diff}");
    }
}
