use crate::client::Client;
use crate::error::Error;
use crate::serde_utils::{
    serialize_as_string_opt, serialize_bool_as_string, serialize_vector_as_string_opt,
};
use derive_builder::Builder;
use serde::Serialize;

#[derive(Debug, Default, Clone, Serialize)]
#[serde(into = "String")]
pub struct Street {
    pub house_number: String,
    pub street_name: String,
}

impl From<Street> for String {
    fn from(street: Street) -> Self {
        format!("{} {}", street.house_number, street.street_name)
    }
}

/// Represents the different types of way that nominatim can request for a
/// location.
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum LocationQuery {
    /// Free-form query string to search for. Free-form queries are
    /// processed first left-to-right and then right-to-left if that fails.
    /// So you may search for `pilkington avenue, birmingham` as well as
    /// for `birmingham, pikington avenue`. Commas are optional but
    /// improve performance by reducing the complexity of the search.
    Generalised { q: String },
    /// Alternative query string format split into several parameters
    /// for structured requests. Structured requests are faster but
    /// are less robust against alternative OSM tagging schemas.
    Structured {
        street: Option<Street>,
        city: Option<String>,
        county: Option<String>,
        state: Option<String>,
        country: Option<String>,
        #[serde(rename = "postalcode")]
        postal_code: Option<String>,
    },
}

#[derive(Builder, Debug, Clone, Serialize)]
pub struct SearchQuery {
    #[serde(flatten)]
    pub location_query: LocationQuery,
    /// Include a breakdown of the address into elements
    #[serde(rename = "addressdetails")]
    #[serde(serialize_with = "serialize_bool_as_string")]
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
    /// Include addition information if the result is available
    /// Limit search results to one of more countries. The country code must
    /// be the
    /// [ISO-3166-1alpha2](https://en.wikipedia.org/wiki/ISO_3166-1_alpha-2)
    /// code, e.g. `gb` for the United Kingdom, `de` for Germany.
    ///
    /// Each place in Nominatim is assigned to one country code based of OSM
    /// country borders. In rare cases a place may not be in any country at
    /// all, for example, in international waters.
    #[builder(default)]
    #[serde(rename = "countrycodes")]
    #[serde(serialize_with = "serialize_vector_as_string_opt")]
    pub country_codes: Option<Vec<String>>,
    /// If you do not want certain OSM objects to appear in the search
    /// result, give a comma separated list of the `place_id`s you want to
    /// skip. This can be used to retrieve additional search results.
    /// For example, if a previous query only returned a few results, then
    /// including those here would cause the search to return other, less
    /// accurate, matches (if possible.)
    #[builder(default)]
    #[serde(serialize_with = "serialize_vector_as_string_opt")]
    pub exclude_place_ids: Option<Vec<u64>>,
    /// Limits the number of returned results. (Default: 10, Maximum: 50.)
    #[builder(default)]
    #[serde(serialize_with = "serialize_as_string_opt")]
    pub limit: Option<u8>,
    /// The preferred area to find search results. Any two corner
    /// points of the box are accepted as long as they span a real box.
    ///
    /// ```http
    /// viewbox=<x1>,<y1>,<x2>,<y2>
    /// ```
    #[builder(default)]
    #[serde(serialize_with = "serialize_vector_as_string_opt")]
    pub viewbox: Option<[f64; 4]>,
    /// Sometimes you have several objects in OSM identifying the same place
    /// or object in reality. The simplest case is a street being split into
    /// many different OSM ways due to different characteristics. Nominatim
    /// will attempt to detect such duplicates and only return on match
    /// unless this parameter is set to `false`. (Default: `true`),
    #[builder(default = "true")]
    #[serde(serialize_with = "serialize_bool_as_string")]
    pub dedupe: bool,
}

impl Client {
    /// The search API allows you to look up a location from a textual
    /// description or addrses. Nominatim supports structured and
    /// free-form search queries.
    pub async fn search(&self, query: SearchQuery) -> Result<geojson::FeatureCollection, Error> {
        self.get_json(
            "search",
            &query,
            &[("format", "geojson"), ("polygon_geojson", "1")],
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Street serialization ─────────────────────────────────────────────────

    #[test]
    fn street_into_string() {
        let s = Street {
            house_number: "10".into(),
            street_name: "Downing St".into(),
        };
        let out: String = s.into();
        assert_eq!(out, "10 Downing St");
    }

    #[test]
    fn street_serializes_as_combined_string() {
        let s = Street {
            house_number: "1".into(),
            street_name: "Parliament Square".into(),
        };
        let v = serde_json::to_value(&s).unwrap();
        assert_eq!(v, "1 Parliament Square");
    }

    // ── LocationQuery → query-string ─────────────────────────────────────────

    fn qs(q: &SearchQuery) -> String {
        serde_urlencoded::to_string(q).unwrap()
    }

    fn base_generalised(q: &str) -> SearchQuery {
        SearchQueryBuilder::default()
            .location_query(LocationQuery::Generalised { q: q.into() })
            .address_details(true)
            .build()
            .unwrap()
    }

    #[test]
    fn generalised_query_produces_q_param() {
        let query = base_generalised("Munich, Germany");
        let qs = qs(&query);
        assert!(
            qs.contains("q=Munich%2C+Germany") || qs.contains("q=Munich%2C%20Germany"),
            "qs: {qs}"
        );
    }

    #[test]
    fn address_details_true_encodes_as_1() {
        let query = base_generalised("Berlin");
        let qs = qs(&query);
        assert!(qs.contains("addressdetails=1"), "qs: {qs}");
    }

    #[test]
    fn address_details_false_encodes_as_0() {
        let query = SearchQueryBuilder::default()
            .location_query(LocationQuery::Generalised { q: "London".into() })
            .address_details(false)
            .build()
            .unwrap();
        let qs = qs(&query);
        assert!(qs.contains("addressdetails=0"), "qs: {qs}");
    }

    #[test]
    fn dedupe_default_true_encodes_as_1() {
        let query = base_generalised("Paris");
        let qs = qs(&query);
        assert!(qs.contains("dedupe=1"), "qs: {qs}");
    }

    #[test]
    fn limit_some_encodes_as_string() {
        let query = SearchQueryBuilder::default()
            .location_query(LocationQuery::Generalised { q: "Rome".into() })
            .address_details(true)
            .limit(Some(50))
            .build()
            .unwrap();
        let qs = qs(&query);
        assert!(qs.contains("limit=50"), "qs: {qs}");
    }

    #[test]
    fn country_codes_encodes_as_comma_string() {
        let query = SearchQueryBuilder::default()
            .location_query(LocationQuery::Generalised { q: "city".into() })
            .address_details(false)
            .country_codes(Some(vec!["gb".into(), "de".into()]))
            .build()
            .unwrap();
        let qs = qs(&query);
        assert!(
            qs.contains("countrycodes=gb%2Cde") || qs.contains("countrycodes=gb,de"),
            "qs: {qs}"
        );
    }

    #[test]
    fn accept_language_encodes_as_comma_string() {
        let query = SearchQueryBuilder::default()
            .location_query(LocationQuery::Generalised { q: "city".into() })
            .address_details(false)
            .accept_language(Some(vec!["en".into(), "fr".into()]))
            .build()
            .unwrap();
        let qs = qs(&query);
        assert!(
            qs.contains("accept-language=en%2Cfr") || qs.contains("accept-language=en,fr"),
            "qs: {qs}"
        );
    }

    #[test]
    fn exclude_place_ids_encodes_as_comma_string() {
        let query = SearchQueryBuilder::default()
            .location_query(LocationQuery::Generalised { q: "city".into() })
            .address_details(false)
            .exclude_place_ids(Some(vec![123, 456]))
            .build()
            .unwrap();
        let qs = qs(&query);
        assert!(
            qs.contains("exclude_place_ids=123%2C456") || qs.contains("exclude_place_ids=123,456"),
            "qs: {qs}"
        );
    }

    #[test]
    fn viewbox_encodes_as_comma_string() {
        let query = SearchQueryBuilder::default()
            .location_query(LocationQuery::Generalised { q: "city".into() })
            .address_details(false)
            .viewbox(Some([-0.5, 51.2, 0.3, 51.8]))
            .build()
            .unwrap();
        let qs = qs(&query);
        assert!(qs.contains("viewbox="), "qs: {qs}");
    }

    #[test]
    fn structured_query_encodes_city_and_country() {
        let query = SearchQueryBuilder::default()
            .location_query(LocationQuery::Structured {
                street: None,
                city: Some("Munich".into()),
                county: None,
                state: None,
                country: Some("Germany".into()),
                postal_code: None,
            })
            .address_details(false)
            .build()
            .unwrap();
        let qs = qs(&query);
        assert!(qs.contains("city=Munich"), "qs: {qs}");
        assert!(qs.contains("country=Germany"), "qs: {qs}");
    }

    // ── geojson FeatureCollection parse ────────────────────────────────────

    const GEOJSON_RESPONSE: &str = r#"{
      "type": "FeatureCollection",
      "licence": "Data © OpenStreetMap contributors, ODbL 1.0.",
      "features": [
        {
          "type": "Feature",
          "properties": {
            "place_id": 282563964,
            "osm_type": "relation",
            "osm_id": "62422",
            "display_name": "Munich, Bavaria, Germany",
            "class": "boundary",
            "type": "administrative",
            "importance": 0.7654
          },
          "bbox": [11.3607562, 48.0617474, 11.7229579, 48.2480911],
          "geometry": {
            "type": "MultiPolygon",
            "coordinates": [[[[11.36,48.06],[11.72,48.06],[11.72,48.25],[11.36,48.25],[11.36,48.06]]]]
          }
        }
      ]
    }"#;

    #[test]
    fn geojson_feature_collection_parses() {
        let fc: geojson::FeatureCollection = serde_json::from_str(GEOJSON_RESPONSE).unwrap();
        assert_eq!(fc.features.len(), 1);
        let feat = &fc.features[0];
        let props = feat.properties.as_ref().unwrap();
        assert_eq!(props["display_name"], "Munich, Bavaria, Germany");
        assert_eq!(props["osm_type"], "relation");
        // geometry is a MultiPolygon
        let geom = feat.geometry.as_ref().unwrap();
        assert!(matches!(
            geom.value,
            geojson::GeometryValue::MultiPolygon { .. }
        ));
    }
}
