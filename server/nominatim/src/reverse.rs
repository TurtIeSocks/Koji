use crate::client::Client;
use crate::error::Error;
use crate::serde_utils::{
    serialize_as_string, serialize_bool_as_string,
    serialize_vector_as_string_opt,
};
use crate::types::Response;
use crate::util::RequestBuilderHelper;
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
    pub async fn reverse(
        &self,
        query: ReverseQuery,
    ) -> Result<Response, Error> {
        let mut url = self.base_url.join("reverse")?;
        url.set_query(Some(&serde_urlencoded::to_string(&query).unwrap()));

        let builder = self.client.get(url).query_s("format", "json");
        let response = builder.send().await?;

        let status = response.status();
        if status != reqwest::StatusCode::OK {
            return Err(Error::ResponseCode(status));
        }

        let text = response.text().await?;

        println!("{}", text);

        Ok(serde_json::from_str(&text)?)
    }
}
