//! Async HTTP client for Dragonite's `/v2/areas/*` API.
//!
//! Reconciled against the real contract (`routes/v2_areas.go`): collection at
//! `/v2/areas/` (trailing slash), single resource at `/v2/areas/{id}`,
//! zero-based `?page` + `?per_page` (max 1000) pagination with a `V2Meta` block,
//! optional `?q=` name filter, and the [`V2Envelope`](crate::envelope) response
//! shape. Every request carries `Authorization: Bearer` + `User-Agent`.

use reqwest::header::USER_AGENT;
use serde::de::DeserializeOwned;

use crate::envelope::{parse_v2, parse_v2_with_meta, V2Meta};
use crate::error::DragoniteError;
use crate::types::ApiArea;

/// Dragonite's documented maximum `per_page`.
const MAX_PER_PAGE: i64 = 1000;

/// Typed client over Dragonite's `/v2/areas` API.
///
/// Construct with [`DragoniteClient::new`]; the `User-Agent` defaults to
/// `koji-dragonite/<crate-version>` and can be overridden with
/// [`DragoniteClient::with_user_agent`].
#[derive(Debug, Clone)]
pub struct DragoniteClient {
    http: reqwest::Client,
    base_url: String,
    bearer: String,
    user_agent: String,
}

impl DragoniteClient {
    /// Build a client for `base_url` authenticating with `bearer`.
    ///
    /// `base_url` should be the scheme+host (optionally with a path prefix),
    /// e.g. `https://dragonite.example`. Trailing slashes are trimmed so path
    /// joining is predictable.
    pub fn new(base_url: impl Into<String>, bearer: impl Into<String>) -> Self {
        let user_agent = format!("koji-dragonite/{}", env!("CARGO_PKG_VERSION"));
        Self {
            http: reqwest::Client::new(),
            base_url: base_url.into().trim_end_matches('/').to_string(),
            bearer: bearer.into(),
            user_agent,
        }
    }

    /// Override the `User-Agent` header (builder-style).
    pub fn with_user_agent(mut self, user_agent: impl Into<String>) -> Self {
        self.user_agent = user_agent.into();
        self
    }

    /// Join the base URL with a path (which should start with `/`).
    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }

    /// Execute a prepared request: attach auth + UA headers, send, then decode
    /// the [`V2Envelope`](crate::envelope) to `(data, meta)`. A non-2xx response
    /// without a decodable envelope surfaces the HTTP status + raw body as an
    /// [`Api`](DragoniteError::Api) error rather than an opaque decode failure.
    async fn exec<T: DeserializeOwned>(
        &self,
        req: reqwest::RequestBuilder,
    ) -> Result<(T, Option<V2Meta>), DragoniteError> {
        let resp = req
            .bearer_auth(&self.bearer)
            .header(USER_AGENT, &self.user_agent)
            .send()
            .await?;
        let status = resp.status();
        let bytes = resp.bytes().await?;
        match parse_v2_with_meta::<T>(&bytes) {
            Ok(ok) => Ok(ok),
            Err(DragoniteError::Decode(_)) if !status.is_success() => Err(DragoniteError::Api {
                code: Some(format!("http_{}", status.as_u16())),
                message: String::from_utf8_lossy(&bytes).trim().to_string(),
                field: None,
            }),
            Err(e) => Err(e),
        }
    }

    /// `exec` discarding the pagination `meta` (single-resource endpoints).
    async fn exec_data<T: DeserializeOwned>(
        &self,
        req: reqwest::RequestBuilder,
    ) -> Result<T, DragoniteError> {
        self.exec(req).await.map(|(data, _)| data)
    }

    /// List one page of areas. `page` is zero-based; `per_page` is capped at
    /// [`MAX_PER_PAGE`]. `q` is an optional case-insensitive name substring
    /// filter. Returns the page plus its [`V2Meta`].
    pub async fn list_areas(
        &self,
        page: i64,
        per_page: i64,
        q: Option<&str>,
    ) -> Result<(Vec<ApiArea>, V2Meta), DragoniteError> {
        let per_page = per_page.clamp(1, MAX_PER_PAGE);
        let mut query: Vec<(&str, String)> =
            vec![("page", page.to_string()), ("per_page", per_page.to_string())];
        if let Some(q) = q.filter(|s| !s.is_empty()) {
            query.push(("q", q.to_string()));
        }
        let req = self.http.get(self.url("/v2/areas/")).query(&query);
        let (data, meta) = self.exec::<Vec<ApiArea>>(req).await?;
        // A list endpoint should always carry meta; synthesize a single-page
        // block if Dragonite ever omits it so callers needn't special-case None.
        let meta = meta.unwrap_or(V2Meta {
            total: data.len() as i64,
            page,
            per_page,
            total_pages: 1,
            has_next: false,
            has_prev: page > 0,
        });
        Ok((data, meta))
    }

    /// Walk every page (zero-based, `per_page = MAX_PER_PAGE`) following
    /// `meta.has_next`, concatenating the results. Uses the server's pagination
    /// metadata as the terminator — no empty-page guessing.
    pub async fn list_all_areas(&self) -> Result<Vec<ApiArea>, DragoniteError> {
        let mut all = Vec::new();
        let mut page = 0;
        loop {
            let (batch, meta) = self.list_areas(page, MAX_PER_PAGE, None).await?;
            all.extend(batch);
            if !meta.has_next {
                break;
            }
            page += 1;
        }
        Ok(all)
    }

    /// Fetch a single area by its Dragonite area id.
    pub async fn get_area(&self, dragonite_area_id: i64) -> Result<ApiArea, DragoniteError> {
        let req = self
            .http
            .get(self.url(&format!("/v2/areas/{dragonite_area_id}")));
        self.exec_data(req).await
    }

    /// Create an area. Send an [`ApiArea`] with `id == None` (the server assigns
    /// it); returns the created area read back from Dragonite.
    pub async fn create_area(&self, area: &ApiArea) -> Result<ApiArea, DragoniteError> {
        let req = self.http.post(self.url("/v2/areas/")).json(area);
        self.exec_data(req).await
    }

    /// PATCH an area, sending only the fields present in `patch` (omitted fields
    /// are left unchanged; a `geofence: Tri::Null` clears that fence). Returns
    /// the updated area. Build `patch` with the
    /// [`area_route_patch`](crate::mapping::area_route_patch) /
    /// [`area_geofence_patch`](crate::mapping::area_geofence_patch) helpers.
    pub async fn patch_area(
        &self,
        dragonite_area_id: i64,
        patch: &ApiArea,
    ) -> Result<ApiArea, DragoniteError> {
        let req = self
            .http
            .patch(self.url(&format!("/v2/areas/{dragonite_area_id}")))
            .json(patch);
        self.exec_data(req).await
    }

    /// Delete an area by its Dragonite area id. Dragonite returns `204 No
    /// Content` on success (no body); a non-2xx surfaces the V2 error envelope.
    pub async fn delete_area(&self, dragonite_area_id: i64) -> Result<(), DragoniteError> {
        let resp = self
            .http
            .delete(self.url(&format!("/v2/areas/{dragonite_area_id}")))
            .bearer_auth(&self.bearer)
            .header(USER_AGENT, &self.user_agent)
            .send()
            .await?;
        let status = resp.status();
        if status.is_success() {
            return Ok(());
        }
        let bytes = resp.bytes().await?;
        // An error response carries the V2 error envelope → surface it.
        match parse_v2::<serde_json::Value>(&bytes) {
            Err(e) => Err(e),
            Ok(_) => Err(DragoniteError::Api {
                code: Some(format!("http_{}", status.as_u16())),
                message: "delete returned a non-2xx status with an `ok` envelope".to_string(),
                field: None,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn url_trims_trailing_slash_on_base() {
        let c = DragoniteClient::new("https://dragonite.example/", "tok");
        assert_eq!(c.url("/v2/areas/"), "https://dragonite.example/v2/areas/");
        assert_eq!(c.url("/v2/areas/7"), "https://dragonite.example/v2/areas/7");
    }

    #[test]
    fn default_user_agent_is_crate_versioned() {
        let c = DragoniteClient::new("https://x", "t");
        assert!(c.user_agent.starts_with("koji-dragonite/"));
    }
}
