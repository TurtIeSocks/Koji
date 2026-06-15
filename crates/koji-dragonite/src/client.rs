//! Async HTTP client for Dragonite's `/v2/areas/*` API.
//!
//! Reconciled against the real contract (`routes/v2_areas.go`): collection at
//! `/v2/areas/` (trailing slash), single resource at `/v2/areas/{id}`,
//! zero-based `?page` + `?per_page` (max 1000) pagination with a `V2Meta` block,
//! optional `?q=` name filter, and the [`V2Envelope`](crate::envelope) response
//! shape. Every request carries `Authorization: Bearer` + `User-Agent`.

use reqwest::header::USER_AGENT;
use serde::de::DeserializeOwned;

use crate::envelope::{V2Meta, parse_v2, parse_v2_with_meta};
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
        let mut query: Vec<(&str, String)> = vec![
            ("page", page.to_string()),
            ("per_page", per_page.to_string()),
        ];
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
    use crate::envelope::{parse_v2, parse_v2_with_meta};
    use crate::types::{ApiArea, ApiLocation};

    // ── construction / config ────────────────────────────────────────────────

    #[test]
    fn url_trims_trailing_slash_on_base() {
        let c = DragoniteClient::new("https://dragonite.example/", "tok");
        assert_eq!(c.url("/v2/areas/"), "https://dragonite.example/v2/areas/");
        assert_eq!(c.url("/v2/areas/7"), "https://dragonite.example/v2/areas/7");
    }

    #[test]
    fn url_no_trailing_slash_on_base_still_joins_correctly() {
        let c = DragoniteClient::new("https://dragonite.example", "tok");
        assert_eq!(c.url("/v2/areas/"), "https://dragonite.example/v2/areas/");
    }

    #[test]
    fn url_multiple_trailing_slashes_are_all_trimmed() {
        let c = DragoniteClient::new("https://dragonite.example///", "tok");
        assert_eq!(c.url("/v2/areas/"), "https://dragonite.example/v2/areas/");
    }

    #[test]
    fn url_path_prefix_is_preserved() {
        // e.g. reverse-proxy with a path prefix
        let c = DragoniteClient::new("https://proxy.example/dragonite/", "tok");
        assert_eq!(
            c.url("/v2/areas/"),
            "https://proxy.example/dragonite/v2/areas/"
        );
        assert_eq!(
            c.url("/v2/areas/42"),
            "https://proxy.example/dragonite/v2/areas/42"
        );
    }

    #[test]
    fn default_user_agent_is_crate_versioned() {
        let c = DragoniteClient::new("https://x", "t");
        assert!(c.user_agent.starts_with("koji-dragonite/"));
    }

    #[test]
    fn with_user_agent_overrides_default() {
        let c = DragoniteClient::new("https://x", "t").with_user_agent("custom-agent/1.0");
        assert_eq!(c.user_agent, "custom-agent/1.0");
    }

    #[test]
    fn with_user_agent_is_builder_returns_client() {
        // Verify the builder pattern actually mutates self (not a no-op clone).
        let c = DragoniteClient::new("https://x", "t").with_user_agent("test/2");
        assert_eq!(c.user_agent, "test/2");
        // Default is gone — no "koji-dragonite/" prefix remains.
        assert!(!c.user_agent.starts_with("koji-dragonite/"));
    }

    #[test]
    fn with_user_agent_accepts_empty_string() {
        // Unusual but valid — no panic.
        let c = DragoniteClient::new("https://x", "t").with_user_agent("");
        assert_eq!(c.user_agent, "");
    }

    #[test]
    fn clone_preserves_all_fields() {
        let orig = DragoniteClient::new("https://dragonite.example/", "secret-token")
            .with_user_agent("agent/99");
        let cloned = orig.clone();
        assert_eq!(cloned.base_url, "https://dragonite.example");
        assert_eq!(cloned.bearer, "secret-token");
        assert_eq!(cloned.user_agent, "agent/99");
    }

    #[test]
    fn debug_repr_contains_base_url() {
        let c = DragoniteClient::new("https://dragonite.example", "tok");
        let dbg = format!("{c:?}");
        assert!(dbg.contains("dragonite.example"));
    }

    // ── URL construction for all API methods ─────────────────────────────────

    #[test]
    fn url_collection_endpoint_has_trailing_slash() {
        // Dragonite collection is at /v2/areas/ (trailing slash is required).
        let c = DragoniteClient::new("https://d.example", "tok");
        assert!(c.url("/v2/areas/").ends_with('/'));
    }

    #[test]
    fn url_area_id_path_format() {
        let c = DragoniteClient::new("https://d.example", "tok");
        // Verify area-id path matches expected format used by get/patch/delete.
        let id: i64 = 42;
        assert_eq!(
            c.url(&format!("/v2/areas/{id}")),
            "https://d.example/v2/areas/42"
        );
    }

    #[test]
    fn url_negative_id_formats_correctly() {
        // Shouldn't happen in practice, but must not panic.
        let c = DragoniteClient::new("https://d.example", "tok");
        assert_eq!(
            c.url(&format!("/v2/areas/{}", -1_i64)),
            "https://d.example/v2/areas/-1"
        );
    }

    // ── response / error mapping (pure JSON → Result, no network) ────────────

    #[test]
    fn parse_v2_area_ok_response() {
        // Verify ApiArea can be decoded through the envelope — exercises the
        // decode path that exec() uses after receiving bytes.
        let body = br#"{"status":"ok","data":{
            "id":7,"name":"test-area","enabled":true,"enable_quests":false
        }}"#;
        let area: ApiArea = parse_v2(body).expect("area ok should parse");
        assert_eq!(area.id, Some(7));
        assert_eq!(area.name.as_deref(), Some("test-area"));
        assert_eq!(area.enabled, Some(true));
        assert_eq!(area.enable_quests, Some(false));
    }

    #[test]
    fn parse_v2_area_list_with_meta() {
        let body = br#"{"status":"ok","data":[
            {"id":1,"name":"a"},{"id":2,"name":"b"}
        ],"meta":{"total":2,"page":0,"per_page":100,"total_pages":1,"has_next":false,"has_prev":false}}"#;
        let (areas, meta) = parse_v2_with_meta::<Vec<ApiArea>>(body).expect("list should parse");
        assert_eq!(areas.len(), 2);
        assert_eq!(areas[0].id, Some(1));
        assert_eq!(areas[1].id, Some(2));
        let meta = meta.expect("meta must be present on list response");
        assert_eq!(meta.total, 2);
        assert_eq!(meta.page, 0);
        assert_eq!(meta.per_page, 100);
        assert!(!meta.has_next);
        assert!(!meta.has_prev);
    }

    #[test]
    fn parse_v2_area_list_multipage_meta() {
        let body = br#"{"status":"ok","data":[
            {"id":101,"name":"x"}
        ],"meta":{"total":50,"page":2,"per_page":20,"total_pages":3,"has_next":false,"has_prev":true}}"#;
        let (areas, meta) = parse_v2_with_meta::<Vec<ApiArea>>(body).expect("should parse");
        assert_eq!(areas.len(), 1);
        let meta = meta.expect("meta must be present");
        assert_eq!(meta.page, 2);
        assert_eq!(meta.per_page, 20);
        assert_eq!(meta.total_pages, 3);
        assert!(!meta.has_next);
        assert!(meta.has_prev);
    }

    #[test]
    fn parse_v2_area_list_has_next_true() {
        // has_next = true means list_all_areas should continue paginating.
        let body = br#"{"status":"ok","data":[{"id":1,"name":"a"}],
            "meta":{"total":5,"page":0,"per_page":1,"total_pages":5,"has_next":true,"has_prev":false}}"#;
        let (_, meta) = parse_v2_with_meta::<Vec<ApiArea>>(body).expect("should parse");
        assert!(meta.expect("meta present").has_next);
    }

    #[test]
    fn parse_v2_empty_list_ok() {
        // Empty data array is valid — no areas.
        let body = br#"{"status":"ok","data":[],
            "meta":{"total":0,"page":0,"per_page":1000,"total_pages":0,"has_next":false,"has_prev":false}}"#;
        let (areas, meta) = parse_v2_with_meta::<Vec<ApiArea>>(body).expect("empty list ok");
        assert!(areas.is_empty());
        let meta = meta.expect("meta present");
        assert_eq!(meta.total, 0);
        assert!(!meta.has_next);
    }

    #[test]
    fn parse_v2_not_found_error_maps_to_api_error() {
        // This is the body exec() would receive on a 404 with a V2 error envelope.
        let body = br#"{"status":"error","error":{"code":"not_found","message":"area not found"}}"#;
        let err = parse_v2::<ApiArea>(body).expect_err("error envelope should fail");
        match err {
            DragoniteError::Api { code, message, field } => {
                assert_eq!(code.as_deref(), Some("not_found"));
                assert_eq!(message, "area not found");
                assert!(field.is_none());
            }
            other => panic!("expected Api, got {other:?}"),
        }
    }

    #[test]
    fn parse_v2_validation_error_with_field() {
        // Per-field validation error carries a field name.
        let body = br#"{"status":"error","error":{"code":"invalid","message":"bad name","field":"name"}}"#;
        let err = parse_v2::<ApiArea>(body).expect_err("should fail");
        match err {
            DragoniteError::Api { code, message, field } => {
                assert_eq!(code.as_deref(), Some("invalid"));
                assert_eq!(message, "bad name");
                assert_eq!(field.as_deref(), Some("name"));
            }
            other => panic!("expected Api, got {other:?}"),
        }
    }

    #[test]
    fn parse_v2_bad_json_is_decode_error() {
        // Non-JSON body (e.g. nginx HTML error page) must surface as Decode.
        let body = b"<html>502 Bad Gateway</html>";
        let err = parse_v2::<ApiArea>(body).expect_err("bad json should fail");
        assert!(matches!(err, DragoniteError::Decode(_)));
    }

    #[test]
    fn parse_v2_wrong_shape_is_decode_error() {
        // Valid JSON but not a V2 envelope.
        let body = br#"{"error":"not found","code":404}"#;
        let err = parse_v2::<ApiArea>(body).expect_err("wrong shape should fail");
        // status field missing → decodes to "" which is unknown → Decode
        assert!(matches!(err, DragoniteError::Decode(_)));
    }

    #[test]
    fn parse_v2_area_with_route_in_pokemon_mode() {
        // Verify that a full area response with nested mode data round-trips.
        let body = br#"{"status":"ok","data":{
            "id":55,"name":"pokearea","enabled":true,
            "pokemon_mode":{
                "workers":3,
                "route":[{"lat":35.1,"lon":139.7},{"lat":35.2,"lon":139.8}]
            }
        }}"#;
        let area: ApiArea = parse_v2(body).expect("should parse");
        assert_eq!(area.id, Some(55));
        let pm = area.pokemon_mode.expect("pokemon_mode present");
        assert_eq!(pm.workers, Some(3));
        assert_eq!(pm.route.len(), 2);
        assert_eq!(pm.route[0], ApiLocation { lat: 35.1, lon: 139.7 });
        assert_eq!(pm.route[1], ApiLocation { lat: 35.2, lon: 139.8 });
    }

    #[test]
    fn parse_v2_delete_ok_envelope_data_is_unit() {
        // delete_area calls parse_v2::<serde_json::Value> on error bodies.
        // Verify an ok-shaped envelope with a JSON object decodes fine.
        let body = br#"{"status":"ok","data":{}}"#;
        let val: serde_json::Value = parse_v2(body).expect("ok data:{} should parse");
        assert!(val.is_object());
    }

    // ── per_page clamping — tested via the MAX_PER_PAGE constant ─────────────

    #[test]
    fn max_per_page_is_1000() {
        // The Dragonite contract specifies max=1000; changing this breaks the API.
        assert_eq!(MAX_PER_PAGE, 1000);
    }

    #[test]
    fn per_page_clamp_low_boundary() {
        // clamp(1, MAX_PER_PAGE): 0 → 1, 1 → 1.
        assert_eq!(0_i64.clamp(1, MAX_PER_PAGE), 1);
        assert_eq!(1_i64.clamp(1, MAX_PER_PAGE), 1);
    }

    #[test]
    fn per_page_clamp_high_boundary() {
        // clamp: 1000 → 1000, 1001 → 1000, i64::MAX → 1000.
        assert_eq!(1000_i64.clamp(1, MAX_PER_PAGE), 1000);
        assert_eq!(1001_i64.clamp(1, MAX_PER_PAGE), 1000);
        assert_eq!(i64::MAX.clamp(1, MAX_PER_PAGE), 1000);
    }

    #[test]
    fn per_page_clamp_negative() {
        // Negative values are invalid → clamped to 1.
        assert_eq!((-5_i64).clamp(1, MAX_PER_PAGE), 1);
    }

    // ── synthesized V2Meta (when server omits it) ─────────────────────────────
    // The actual synthesis is inline in list_areas (async, needs HTTP), but we
    // can verify the V2Meta fields we construct are logically consistent.

    #[test]
    fn synthesized_meta_for_page_0_has_no_prev() {
        let data_len = 3_i64;
        let page = 0_i64;
        let per_page = 50_i64;
        let synth = V2Meta {
            total: data_len,
            page,
            per_page,
            total_pages: 1,
            has_next: false,
            has_prev: page > 0,
        };
        assert!(!synth.has_prev);
        assert!(!synth.has_next);
        assert_eq!(synth.total, 3);
    }

    #[test]
    fn synthesized_meta_for_page_1_has_prev() {
        let page = 1_i64;
        let synth = V2Meta {
            total: 10,
            page,
            per_page: 10,
            total_pages: 2,
            has_next: false,
            has_prev: page > 0,
        };
        assert!(synth.has_prev);
    }
}
