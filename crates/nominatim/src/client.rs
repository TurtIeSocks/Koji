use crate::util::RequestBuilderHelper;
use thiserror::Error;

/// A nominatim client that is binded to the nominatim web api.
#[derive(Clone)]
pub struct Client {
    /// The user agent of your service. This is required by the Nominatim
    /// terms of service.
    ///
    /// Note that changing it does nothing unless respecified in the client.
    pub user_agent: String,
    /// ***Strongly Recommended***, your email so Nominatim can contact you
    /// in case they dislike your usecase.
    pub email: Option<String>,
    /// The base URL
    pub base_url: reqwest::Url,
    pub client: reqwest::Client,
}

/// An error that may be returned when creating a new
/// client.
#[derive(Error, Debug)]
pub enum NewError {
    #[error("reqwest error: {0}")]
    Reqwest(#[from] reqwest::Error),
}

impl Client {
    /// Creates a new client base
    ///
    /// # User Agent
    ///
    /// The user agent of your service. This is required by the Nominatim
    /// terms of service.
    ///
    /// # Email
    ///
    /// ***Strongly Recommended***, your email so Nominatim can contact you
    /// in case they dislike your usecase.
    pub fn new(
        base_url: reqwest::Url,
        user_agent: String,
        email: Option<String>,
    ) -> Result<Self, NewError> {
        Ok(Self {
            client: reqwest::Client::builder().user_agent(&user_agent).build()?,
            base_url,
            user_agent,
            email,
        })
    }

    /// Shared request→status-check→parse skeleton for the endpoint methods.
    ///
    /// Joins `path` onto the base URL, urlencodes `query` into the query
    /// string, appends any `extra` key/value pairs (e.g. `format`), sends the
    /// GET, errors on any non-200 status, and parses the body as JSON.
    pub(crate) async fn get_json<R: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        query: &impl serde::Serialize,
        extra: &[(&str, &str)],
    ) -> Result<R, crate::error::Error> {
        let mut url = self.base_url.join(path)?;
        url.set_query(Some(&serde_urlencoded::to_string(query)?));
        let mut builder = self.client.get(url);
        for (k, v) in extra {
            builder = builder.query_s(k, v);
        }
        let resp = builder.send().await?;
        let status = resp.status();
        if status != reqwest::StatusCode::OK {
            return Err(crate::error::Error::ResponseCode(status));
        }
        Ok(serde_json::from_str(&resp.text().await?)?)
    }
}
