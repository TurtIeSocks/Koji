use crate::client::Client;
use crate::error::Error;
use crate::serde_utils::{
    serialize_as_string_opt, serialize_bool_as_string, serialize_vector_as_string_opt,
};
use crate::util::RequestBuilderHelper;
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
        let mut url = self.base_url.join("search")?;
        url.set_query(Some(&serde_urlencoded::to_string(&query).unwrap()));

        let builder = self
            .client
            .get(url)
            .query_s("format", "geojson")
            .query_s("polygon_geojson", "1");
        let response = builder.send().await?;

        let status = response.status();
        if status != reqwest::StatusCode::OK {
            return Err(Error::ResponseCode(status));
        }

        let text = response.text().await?;

        Ok(serde_json::from_str(&text)?)
    }
}
