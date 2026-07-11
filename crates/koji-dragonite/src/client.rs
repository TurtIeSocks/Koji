//! Async HTTP client for Dragonite's `/v2/areas/*` API.
//!
//! Reconciled against the real contract (`routes/v2_areas.go`): single resource
//! at `/v2/areas/{id}` patched via PATCH, decoded through the
//! [`V2Envelope`](crate::envelope) response shape. Every request carries
//! `Authorization: Bearer` + `User-Agent`.

use reqwest::header::USER_AGENT;
use serde::de::DeserializeOwned;

use crate::envelope::parse_v2;
use crate::error::DragoniteError;
use crate::types::ApiArea;

/// Typed client over Dragonite's `/v2/areas` API.
///
/// Construct with [`DragoniteClient::new`]; the `User-Agent` defaults to
/// `koji-dragonite/<crate-version>`.
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

    /// Join the base URL with a path (which should start with `/`).
    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }

    /// Execute a prepared request: attach auth + UA headers, send, then decode
    /// the [`V2Envelope`](crate::envelope) to its `data`. A non-2xx response
    /// without a decodable envelope surfaces the typed
    /// [`HttpStatus`](DragoniteError::HttpStatus) error rather than an opaque
    /// decode failure.
    async fn exec<T: DeserializeOwned>(
        &self,
        req: reqwest::RequestBuilder,
    ) -> Result<T, DragoniteError> {
        let resp = req
            .bearer_auth(&self.bearer)
            .header(USER_AGENT, &self.user_agent)
            .send()
            .await?;
        let status = resp.status();
        let bytes = resp.bytes().await?;
        match parse_v2::<T>(&bytes) {
            Ok(ok) => Ok(ok),
            Err(DragoniteError::Decode(_)) if !status.is_success() => {
                Err(DragoniteError::HttpStatus {
                    status: status.as_u16(),
                    body: String::from_utf8_lossy(&bytes).trim().to_string(),
                })
            }
            Err(e) => Err(e),
        }
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
        self.exec(req).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::envelope::parse_v2;
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
    fn clone_preserves_all_fields() {
        let orig = DragoniteClient::new("https://dragonite.example/", "secret-token");
        let cloned = orig.clone();
        assert_eq!(cloned.base_url, "https://dragonite.example");
        assert_eq!(cloned.bearer, "secret-token");
        assert!(cloned.user_agent.starts_with("koji-dragonite/"));
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
        // Verify area-id path matches expected format used by patch_area.
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
    fn parse_v2_list_with_meta_block_ignores_meta() {
        // A list body carrying the (now-unread) meta block still parses; the
        // pagination machinery was deleted with no list endpoint shipped.
        let body = br#"{"status":"ok","data":[
            {"id":1,"name":"a"},{"id":2,"name":"b"}
        ],"meta":{"total":2,"page":0,"per_page":100,"total_pages":1,"has_next":false,"has_prev":false}}"#;
        let areas = parse_v2::<Vec<ApiArea>>(body).expect("list should parse");
        assert_eq!(areas.len(), 2);
        assert_eq!(areas[0].id, Some(1));
    }

    #[test]
    fn parse_v2_empty_list_ok() {
        let body = br#"{"status":"ok","data":[],
            "meta":{"total":0,"page":0,"per_page":1000,"total_pages":0,"has_next":false,"has_prev":false}}"#;
        let areas = parse_v2::<Vec<ApiArea>>(body).expect("empty list ok");
        assert!(areas.is_empty());
    }

    #[test]
    fn parse_v2_not_found_error_maps_to_api_error() {
        // This is the body exec() would receive on a 404 with a V2 error envelope.
        let body = br#"{"status":"error","error":{"code":"not_found","message":"area not found"}}"#;
        let err = parse_v2::<ApiArea>(body).expect_err("error envelope should fail");
        match err {
            DragoniteError::Api {
                code,
                message,
                field,
            } => {
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
        let body =
            br#"{"status":"error","error":{"code":"invalid","message":"bad name","field":"name"}}"#;
        let err = parse_v2::<ApiArea>(body).expect_err("should fail");
        match err {
            DragoniteError::Api {
                code,
                message,
                field,
            } => {
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
        assert_eq!(
            pm.route[0],
            ApiLocation {
                lat: 35.1,
                lon: 139.7
            }
        );
        assert_eq!(
            pm.route[1],
            ApiLocation {
                lat: 35.2,
                lon: 139.8
            }
        );
    }

    #[test]
    fn parse_v2_ok_envelope_data_is_unit() {
        // Verify an ok-shaped envelope with a JSON object decodes fine.
        let body = br#"{"status":"ok","data":{}}"#;
        let val: serde_json::Value = parse_v2(body).expect("ok data:{} should parse");
        assert!(val.is_object());
    }
}
