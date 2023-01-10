use crate::client::Client;
use crate::error::Error;
use crate::serde_utils::{
    serialize_as_string_opt, serialize_bool_as_string, serialize_vector_as_string,
    serialize_vector_as_string_opt,
};
use crate::types::Response;
use crate::util::RequestBuilderHelper;
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
        let mut url = self.base_url.join("lookup")?;
        url.set_query(Some(&serde_urlencoded::to_string(&query).unwrap()));

        let builder = self.client.get(url).query_s("format", "json");
        let response = builder.send().await?;

        let status = response.status();
        if status != reqwest::StatusCode::OK {
            return Err(Error::ResponseCode(status));
        }

        let text = response.text().await?;

        Ok(serde_json::from_str(&text)?)
    }
}
