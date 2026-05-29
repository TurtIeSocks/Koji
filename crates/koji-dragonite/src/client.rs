//! Async HTTP client for Dragonite's `/v2/areas/*` API.
//!
//! The PLUMBING here is sound: reqwest client construction, `Authorization:
//! Bearer` + `User-Agent` headers, JSend → `Result` collapsing, and the
//! page-walking loop. The ENDPOINT PATHS and request BODIES are PROVISIONAL —
//! every request carries a `// TODO(dragonite-reconcile): ...` marker. None of
//! this has been run against a live Dragonite (all HTTP is runtime-unverified).

use reqwest::Method;
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::error::DragoniteError;
use crate::jsend::parse_jsend;
use crate::types::{ApiArea, V2GeofencePatch};

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

    /// Issue a request with auth + UA headers, then collapse the JSend body to a
    /// `Result`. `body` is sent as JSON when `Some`. This is the single choke
    /// point through which every method goes, so headers + envelope handling
    /// live in exactly one place.
    async fn send<B, T>(
        &self,
        method: Method,
        path: &str,
        body: Option<&B>,
    ) -> Result<T, DragoniteError>
    where
        B: Serialize + ?Sized,
        T: DeserializeOwned,
    {
        let mut req = self
            .http
            .request(method, self.url(path))
            .bearer_auth(&self.bearer)
            .header(reqwest::header::USER_AGENT, &self.user_agent);

        if let Some(b) = body {
            req = req.json(b);
        }

        let bytes = req.send().await?.bytes().await?;
        parse_jsend::<T>(&bytes)
    }

    /// List one page of areas.
    ///
    /// Pagination is by `page` (0- or 1-based — unverified). Returns the areas
    /// on that page; an empty `Vec` signals the end of the listing to
    /// [`list_all_areas`](Self::list_all_areas).
    ///
    // TODO(dragonite-reconcile): verify path + pagination — `/v2/areas?page=N`
    // is a guess; the real param may be `page`/`offset`/`cursor`, and the page
    // index base + page size are unknown. The JSend `data` is assumed to be a
    // bare array of areas; Dragonite may instead nest it (e.g. `data.areas`)
    // with paging metadata in `meta`.
    pub async fn list_areas(&self, page: u32) -> Result<Vec<ApiArea>, DragoniteError> {
        let path = format!("/v2/areas?page={page}");
        self.send::<(), Vec<ApiArea>>(Method::GET, &path, None)
            .await
    }

    /// Walk every page via [`list_areas`](Self::list_areas) until a page comes
    /// back empty, concatenating the results.
    ///
    // TODO(dragonite-reconcile): the empty-page terminator assumes a 1-based
    // (or 0-based) sequential pager that returns `[]` past the end. Re-check
    // once the real pagination contract is known to avoid an infinite loop or a
    // premature stop.
    pub async fn list_all_areas(&self) -> Result<Vec<ApiArea>, DragoniteError> {
        let mut all = Vec::new();
        let mut page = 1;
        loop {
            let batch = self.list_areas(page).await?;
            if batch.is_empty() {
                break;
            }
            all.extend(batch);
            page += 1;
        }
        Ok(all)
    }

    /// Fetch a single area by its Dragonite area id.
    ///
    // TODO(dragonite-reconcile): verify path `/v2/areas/{id}`.
    pub async fn get_area(&self, dragonite_area_id: u32) -> Result<ApiArea, DragoniteError> {
        let path = format!("/v2/areas/{dragonite_area_id}");
        self.send::<(), ApiArea>(Method::GET, &path, None).await
    }

    /// Create an area.
    ///
    // TODO(dragonite-reconcile): verify path `/v2/areas` and the create request
    // body shape. We send the provisional `ApiArea` as the body, which is almost
    // certainly not the real create payload (creates usually omit the
    // server-assigned id). Treat this as a placeholder.
    pub async fn create_area(&self, area: &ApiArea) -> Result<ApiArea, DragoniteError> {
        self.send::<ApiArea, ApiArea>(Method::POST, "/v2/areas", Some(area))
            .await
    }

    /// PATCH an area's geofences, sending ONLY the fields present in `patch`
    /// (its [`Tri`](crate::patch::Tri) fields handle the absent/null/value
    /// distinction). An empty patch is a no-op — it would serialize to `{}` —
    /// but is still sent; callers can guard with
    /// [`V2GeofencePatch::is_empty`](crate::types::V2GeofencePatch::is_empty).
    ///
    // TODO(dragonite-reconcile): verify path `/v2/areas/{id}` for PATCH and the
    // patch body field names (see `V2GeofencePatch`). The tri-state *mechanics*
    // are correct; the *keys* are provisional.
    pub async fn patch_area(
        &self,
        dragonite_area_id: u32,
        patch: &V2GeofencePatch,
    ) -> Result<ApiArea, DragoniteError> {
        let path = format!("/v2/areas/{dragonite_area_id}");
        self.send::<V2GeofencePatch, ApiArea>(Method::PATCH, &path, Some(patch))
            .await
    }

    /// Delete an area by its Dragonite area id.
    ///
    // TODO(dragonite-reconcile): verify path `/v2/areas/{id}` for DELETE and
    // whether the success body is empty/`null`. We decode `data` as
    // `serde_json::Value` to tolerate either a payload or `null`.
    pub async fn delete_area(
        &self,
        dragonite_area_id: u32,
    ) -> Result<serde_json::Value, DragoniteError> {
        let path = format!("/v2/areas/{dragonite_area_id}");
        self.send::<(), serde_json::Value>(Method::DELETE, &path, None)
            .await
    }
}
