use crate::client::Client;
use crate::error::Error;
use crate::serde_utils::{
    serialize_as_string, serialize_bool_as_string, serialize_vector_as_string_opt,
};
use crate::types::Response;
use derive_builder::Builder;
use serde::Serialize;
use std::fmt;

#[derive(Debug, Clone)]
pub enum Zoom {
    Country,
    State,
    County,
    City,
    Suburb,
    MajorStreets,
    MajorAndMinorStreets,
    Building,
}

impl fmt::Display for Zoom {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Country => write!(f, "3"),
            Self::State => write!(f, "5"),
            Self::County => write!(f, "8"),
            Self::City => write!(f, "10"),
            Self::Suburb => write!(f, "14"),
            Self::MajorStreets => write!(f, "16"),
            Self::MajorAndMinorStreets => write!(f, "17"),
            Self::Building => write!(f, "18"),
        }
    }
}

#[derive(Builder, Debug, Clone, Serialize)]
pub struct ReverseQuery {
    #[serde(serialize_with = "serialize_as_string")]
    pub lat: f64,

    #[serde(serialize_with = "serialize_as_string")]
    pub lon: f64,

    /// Include a breakdown of the address into elements. (Default: true)
    #[serde(rename = "addressdetails")]
    #[serde(serialize_with = "serialize_bool_as_string")]
    #[builder(default = "true")]
    pub address_details: bool,

    /// Include additional information if the result is available
    #[builder(default)]
    #[serde(rename = "extratags")]
    #[serde(serialize_with = "serialize_bool_as_string")]
    pub extra_tags: bool,

    /// Include a list of alternative names in the results. This may include
    /// language variants, references, operator and brand.
    #[builder(default)]
    #[serde(rename = "namedetails")]
    #[serde(serialize_with = "serialize_bool_as_string")]
    pub name_details: bool,

    /// Preferred language order for showing search results, overrides
    /// the value specified in the "Accept-Languague" HTTP header.
    /// Either use a standard RFC2616 accept-language string or
    /// a simple comma-separated list of language codes.
    #[builder(default)]
    #[serde(rename = "accept-language")]
    #[serde(serialize_with = "serialize_vector_as_string_opt")]
    pub accept_language: Option<Vec<String>>,

    #[serde(serialize_with = "serialize_as_string")]
    pub zoom: Zoom,
}

impl Client {
    /// Reverse geocoding generates an address from a latitude and
    /// longitude.
    ///
    /// ## How it works
    ///
    /// The reverse geocoding API does not exactly compute the address for the
    /// coordinate it receives. It works by finding the closest suitable OSM
    /// object and returning its address information. This may occasionally lead
    /// to unexpected results.
    ///
    /// First of all, Nominatim only includes OSM objects in its index that are
    /// suitable for searching. Small, unnamed paths for example are missing
    /// from the database and can therefore not be used for reverse geocoding
    /// either.
    ///
    /// The other issue to be aware of is that the closest OSM object may not
    /// always have a similar enough address to the coordinate you were
    /// requesting. For example, in dense city areas it may belong to a
    /// completely different street.
    pub async fn reverse(&self, query: ReverseQuery) -> Result<Response, Error> {
        self.get_json("reverse", &query, &[("format", "json")])
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Zoom display ─────────────────────────────────────────────────────────

    #[test]
    fn zoom_display_values() {
        assert_eq!(Zoom::Country.to_string(), "3");
        assert_eq!(Zoom::State.to_string(), "5");
        assert_eq!(Zoom::County.to_string(), "8");
        assert_eq!(Zoom::City.to_string(), "10");
        assert_eq!(Zoom::Suburb.to_string(), "14");
        assert_eq!(Zoom::MajorStreets.to_string(), "16");
        assert_eq!(Zoom::MajorAndMinorStreets.to_string(), "17");
        assert_eq!(Zoom::Building.to_string(), "18");
    }

    // ── ReverseQuery → query-string ──────────────────────────────────────────

    fn qs(q: &ReverseQuery) -> String {
        serde_urlencoded::to_string(q).unwrap()
    }

    fn base_query() -> ReverseQuery {
        ReverseQueryBuilder::default()
            .lat(48.1371)
            .lon(11.5754)
            .zoom(Zoom::City)
            .build()
            .unwrap()
    }

    #[test]
    fn lat_lon_encode_as_strings() {
        let qs = qs(&base_query());
        // serde serializes f64 via Display; check both keys present
        assert!(qs.contains("lat="), "qs: {qs}");
        assert!(qs.contains("lon="), "qs: {qs}");
    }

    #[test]
    fn zoom_encodes_as_numeric_string() {
        let qs = qs(&base_query());
        assert!(qs.contains("zoom=10"), "qs: {qs}");
    }

    #[test]
    fn address_details_default_true_encodes_1() {
        let qs = qs(&base_query());
        assert!(qs.contains("addressdetails=1"), "qs: {qs}");
    }

    #[test]
    fn address_details_false_encodes_0() {
        let q = ReverseQueryBuilder::default()
            .lat(0.0)
            .lon(0.0)
            .zoom(Zoom::Country)
            .address_details(false)
            .build()
            .unwrap();
        let qs = qs(&q);
        assert!(qs.contains("addressdetails=0"), "qs: {qs}");
    }

    #[test]
    fn accept_language_some_encodes() {
        let q = ReverseQueryBuilder::default()
            .lat(0.0)
            .lon(0.0)
            .zoom(Zoom::Building)
            .accept_language(Some(vec!["de".into()]))
            .build()
            .unwrap();
        let qs = qs(&q);
        assert!(qs.contains("accept-language=de"), "qs: {qs}");
    }

    // ── Response JSON parsing (same struct used by reverse endpoint) ──────────

    const REVERSE_JSON: &str = r#"{
      "place_id": 12345,
      "osm_type": "way",
      "osm_id": 99999,
      "lat": "48.1371",
      "lon": "11.5754",
      "display_name": "Marienplatz, Munich, Bavaria, Germany",
      "class": "highway",
      "type": "pedestrian",
      "importance": 0.5,
      "boundingbox": ["48.136","48.138","11.574","11.576"]
    }"#;

    #[test]
    fn reverse_response_parses() {
        use crate::types::{ID, OsmType, Response};
        let r: Response = serde_json::from_str(REVERSE_JSON).unwrap();
        assert_eq!(r.place_id, Some(ID::Num(12345)));
        assert_eq!(r.osm_type, Some(OsmType::Way));
        assert!((r.lat.unwrap() - 48.1371).abs() < 1e-4);
        assert!((r.lon.unwrap() - 11.5754).abs() < 1e-4);
        assert_eq!(r.class.as_deref(), Some("highway"));
    }
}
