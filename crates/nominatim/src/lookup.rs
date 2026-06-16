use crate::client::Client;
use crate::error::Error;
use crate::serde_utils::{
    serialize_as_string_opt, serialize_bool_as_string, serialize_vector_as_string,
    serialize_vector_as_string_opt,
};
use crate::types::Response;
use derive_builder::Builder;
use serde::Serialize;

#[derive(Builder, Debug, Clone, Serialize)]
pub struct LookupQuery {
    /// `osm_ids` is mandatory and must contain a comma-seperated list of
    /// OSM ids each prefixed with its type, on of node(N), way(W) or
    /// relation(R). Up to 50 ids can be queried at the same time.
    #[builder(default)]
    #[serde(serialize_with = "serialize_vector_as_string")]
    pub osm_ids: Vec<String>,
    /// Include a breakdown of the address into elements. (Default: false)
    #[serde(rename = "addressdetails")]
    #[serde(serialize_with = "serialize_bool_as_string")]
    #[builder(default)]
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
    /// When a viewbox is given, restrict the result to items contained
    /// within the viewbox (see above). When `viewbox` and `bounded = true`
    /// are given, an amenity only search is allowed. Give the special keyword
    /// for the amenity in square brackets, e.g. `[pub]` and a selection of
    /// objects of this type is returned. There is no guarantee that the result
    /// is complete. (Default: 0)
    #[builder(default)]
    #[serde(serialize_with = "serialize_bool_as_string")]
    pub bounded: bool,
}

impl Client {
    /// The lookup API allows to query the address and other details of one or
    /// multiple OSM objects like node, way or relation.
    pub async fn lookup(&self, query: LookupQuery) -> Result<Vec<Response>, Error> {
        self.get_json("lookup", &query, &[("format", "json")]).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn qs(q: &LookupQuery) -> String {
        serde_urlencoded::to_string(q).unwrap()
    }

    fn base_query(ids: Vec<String>) -> LookupQuery {
        LookupQueryBuilder::default().osm_ids(ids).build().unwrap()
    }

    // ── osm_ids encodes as comma-joined string ─────────────────────────────

    #[test]
    fn osm_ids_encode_as_comma_string() {
        let q = base_query(vec!["N1".into(), "W2".into(), "R3".into()]);
        let qs = qs(&q);
        assert!(
            qs.contains("osm_ids=N1%2CW2%2CR3") || qs.contains("osm_ids=N1,W2,R3"),
            "qs: {qs}"
        );
    }

    #[test]
    fn empty_osm_ids_encodes_empty_string() {
        let q = base_query(vec![]);
        let qs = qs(&q);
        assert!(qs.contains("osm_ids="), "qs: {qs}");
    }

    // ── bool flags ────────────────────────────────────────────────────────────

    #[test]
    fn address_details_false_by_default_encodes_0() {
        let q = base_query(vec!["N1".into()]);
        let qs = qs(&q);
        assert!(qs.contains("addressdetails=0"), "qs: {qs}");
    }

    #[test]
    fn address_details_true_encodes_1() {
        let q = LookupQueryBuilder::default()
            .osm_ids(vec!["N1".into()])
            .address_details(true)
            .build()
            .unwrap();
        let qs = qs(&q);
        assert!(qs.contains("addressdetails=1"), "qs: {qs}");
    }

    #[test]
    fn bounded_false_by_default_encodes_0() {
        let q = base_query(vec!["R1".into()]);
        let qs = qs(&q);
        assert!(qs.contains("bounded=0"), "qs: {qs}");
    }

    #[test]
    fn limit_some_encodes_as_number_string() {
        let q = LookupQueryBuilder::default()
            .osm_ids(vec!["N1".into()])
            .limit(Some(10))
            .build()
            .unwrap();
        let qs = qs(&q);
        assert!(qs.contains("limit=10"), "qs: {qs}");
    }

    #[test]
    fn country_codes_some_encodes_comma_string() {
        let q = LookupQueryBuilder::default()
            .osm_ids(vec!["N1".into()])
            .country_codes(Some(vec!["gb".into(), "us".into()]))
            .build()
            .unwrap();
        let qs = qs(&q);
        assert!(
            qs.contains("countrycodes=gb%2Cus") || qs.contains("countrycodes=gb,us"),
            "qs: {qs}"
        );
    }

    // ── Vec<Response> JSON parse ────────────────────────────────────────────

    const LOOKUP_JSON: &str = r#"[
      {
        "place_id": 100,
        "osm_type": "node",
        "osm_id": 200,
        "lat": "51.5074",
        "lon": "-0.1278",
        "display_name": "London, England",
        "class": "place",
        "type": "city",
        "importance": 0.9,
        "boundingbox": ["51.28","51.69","-0.51","0.33"]
      }
    ]"#;

    #[test]
    fn lookup_response_vec_parses() {
        use crate::types::{ID, OsmType, Response};
        let results: Vec<Response> = serde_json::from_str(LOOKUP_JSON).unwrap();
        assert_eq!(results.len(), 1);
        let r = &results[0];
        assert_eq!(r.place_id, Some(ID::Num(100)));
        assert_eq!(r.osm_type, Some(OsmType::Node));
        assert_eq!(r.display_name.as_deref(), Some("London, England"));
    }
}
