//! Async HTTP client for Dragonite's `/v2/areas/*` API.
//!
//! Single resource at `/v2/areas/{id}` patched via PATCH. Dragonite answers a
//! success with the bare area and a failure with a problem document (checked
//! against Dragonite 1.19.0), read by [`decode_reply`]. Every request carries
//! `Authorization: Bearer` + `User-Agent`.

use reqwest::header::USER_AGENT;
use serde::de::DeserializeOwned;

use crate::error::{DragoniteError, ProblemDocument};
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

    /// Execute a prepared request: attach auth + UA headers, send, then read the
    /// reply with [`decode_reply`].
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
        decode_reply(status.as_u16(), &bytes)
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

/// Read one Dragonite reply from its HTTP `status` and raw `body`.
///
/// - 2xx: the body is the bare payload (no envelope) and decodes straight into
///   `T`; a body that does not is [`DragoniteError::Decode`].
/// - anything else: [`DragoniteError::HttpStatus`] carrying `status`, the
///   problem document's `detail` and `errors` when the body is one, and the raw
///   text. An empty or non-JSON body still yields the typed error.
fn decode_reply<T: DeserializeOwned>(status: u16, body: &[u8]) -> Result<T, DragoniteError> {
    if (200..300).contains(&status) {
        return serde_json::from_slice(body).map_err(|e| DragoniteError::Decode(e.to_string()));
    }
    let problem: ProblemDocument = serde_json::from_slice(body).unwrap_or_default();
    Err(DragoniteError::HttpStatus {
        status,
        detail: problem.detail,
        errors: problem.errors,
        body: String::from_utf8_lossy(body).trim().to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
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

    // ── reply decoding (pure status + bytes → Result, no network) ────────────

    /// A 200 from `PATCH /v2/areas/2`, as Dragonite 1.19.0 sends it.
    const BARE_AREA: &[u8] = br#"{
        "$schema":"https://dragonite.example/schemas/ApiAreaV2.json",
        "id":2,"name":"CT Ansonia","enabled":false,
        "pokemon_mode":{"workers":3,"route":[{"lat":35.1,"lon":139.7},{"lat":35.2,"lon":139.8}]},
        "quest_mode":{},"fort_mode":{}
    }"#;

    /// A 422 from `PATCH /v2/areas/2 {"enabled":"x"}`: a problem document whose
    /// `status` is a number.
    const PROBLEM_422: &[u8] = br#"{
        "$schema":"https://dragonite.example/schemas/ErrorModel.json",
        "title":"Unprocessable Entity","status":422,"detail":"validation failed",
        "errors":[{"message":"expected boolean","location":"body.enabled","value":"x"}]
    }"#;

    #[test]
    fn decode_reply_bare_area_is_a_success() {
        let area: ApiArea = decode_reply(200, BARE_AREA).expect("bare area decodes");
        assert_eq!(area.id, Some(2));
        assert_eq!(area.name.as_deref(), Some("CT Ansonia"));
        assert_eq!(area.enabled, Some(false));
        let pm = area.pokemon_mode.expect("pokemon_mode present");
        assert_eq!(pm.workers, Some(3));
        assert_eq!(
            pm.route,
            vec![
                ApiLocation {
                    lat: 35.1,
                    lon: 139.7
                },
                ApiLocation {
                    lat: 35.2,
                    lon: 139.8
                }
            ]
        );
    }

    #[test]
    fn decode_reply_422_problem_document_is_a_typed_error_with_its_detail() {
        let err = decode_reply::<ApiArea>(422, PROBLEM_422).expect_err("422 is an error");
        match err {
            DragoniteError::HttpStatus {
                status,
                detail,
                errors,
                ..
            } => {
                assert_eq!(status, 422);
                assert_eq!(detail.as_deref(), Some("validation failed"));
                assert_eq!(errors.len(), 1);
                assert_eq!(errors[0].message.as_deref(), Some("expected boolean"));
                assert_eq!(errors[0].location.as_deref(), Some("body.enabled"));
                assert_eq!(errors[0].value, Some(serde_json::json!("x")));
            }
            other => panic!("expected HttpStatus, got {other:?}"),
        }
    }

    #[test]
    fn decode_reply_empty_500_is_a_typed_error() {
        let err = decode_reply::<ApiArea>(500, b"").expect_err("500 is an error");
        match err {
            DragoniteError::HttpStatus {
                status,
                detail,
                errors,
                body,
            } => {
                assert_eq!(status, 500);
                assert_eq!(detail, None);
                assert!(errors.is_empty());
                assert_eq!(body, "");
            }
            other => panic!("expected HttpStatus, got {other:?}"),
        }
    }

    #[test]
    fn decode_reply_non_json_error_body_keeps_the_text() {
        let err = decode_reply::<ApiArea>(502, b"<html>502 Bad Gateway</html>\n")
            .expect_err("502 is an error");
        match err {
            DragoniteError::HttpStatus {
                status,
                detail,
                body,
                ..
            } => {
                assert_eq!(status, 502);
                assert_eq!(detail, None);
                assert_eq!(body, "<html>502 Bad Gateway</html>");
            }
            other => panic!("expected HttpStatus, got {other:?}"),
        }
    }

    #[test]
    fn decode_reply_2xx_with_a_body_that_is_not_an_area_is_a_decode_error() {
        let err = decode_reply::<ApiArea>(200, b"<html>hi</html>").expect_err("not JSON");
        assert!(matches!(err, DragoniteError::Decode(_)));
    }

    // ── patch_area against a local HTTP stand-in ─────────────────────────────

    /// Serve one canned reply on a loopback port. Returns the base URL and a
    /// handle yielding the raw request (head and body) the client sent.
    async fn serve_once(
        status_line: &'static str,
        body: &'static [u8],
    ) -> (String, tokio::task::JoinHandle<String>) {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let handle = tokio::spawn(async move {
            let (mut sock, _) = listener.accept().await.unwrap();
            let mut req = Vec::new();
            let mut buf = [0u8; 4096];
            loop {
                let n = sock.read(&mut buf).await.unwrap();
                req.extend_from_slice(&buf[..n]);
                let text = String::from_utf8_lossy(&req).to_string();
                if let Some(head_end) = text.find("\r\n\r\n") {
                    let want = text[..head_end]
                        .lines()
                        .find_map(|l| {
                            l.to_ascii_lowercase()
                                .strip_prefix("content-length:")
                                .and_then(|v| v.trim().parse::<usize>().ok())
                        })
                        .unwrap_or(0);
                    if req.len() >= head_end + 4 + want || n == 0 {
                        break;
                    }
                } else if n == 0 {
                    break;
                }
            }
            let head = format!(
                "HTTP/1.1 {status_line}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n",
                body.len()
            );
            sock.write_all(head.as_bytes()).await.unwrap();
            sock.write_all(body).await.unwrap();
            sock.shutdown().await.unwrap();
            String::from_utf8_lossy(&req).to_string()
        });
        (base, handle)
    }

    #[tokio::test]
    async fn patch_area_returns_the_bare_area_dragonite_answers_with() {
        let (base, request) = serve_once("200 OK", BARE_AREA).await;
        let client = DragoniteClient::new(base, "tok");
        let patch = ApiArea {
            enabled: Some(false),
            ..ApiArea::default()
        };

        let area = client
            .patch_area(2, &patch)
            .await
            .expect("200 is a success");

        assert_eq!(area.id, Some(2));
        assert_eq!(area.name.as_deref(), Some("CT Ansonia"));
        let request = request.await.unwrap();
        assert!(request.starts_with("PATCH /v2/areas/2 "), "{request}");
        assert!(
            request
                .to_ascii_lowercase()
                .contains("authorization: bearer tok"),
            "{request}"
        );
        assert!(request.contains(r#"{"enabled":false}"#), "{request}");
    }

    #[tokio::test]
    async fn patch_area_422_surfaces_the_problem_documents_detail() {
        let (base, _request) = serve_once("422 Unprocessable Entity", PROBLEM_422).await;
        let client = DragoniteClient::new(base, "tok");

        let err = client
            .patch_area(2, &ApiArea::default())
            .await
            .expect_err("422 is an error");

        assert!(
            matches!(&err, DragoniteError::HttpStatus { status: 422, detail: Some(d), .. } if d == "validation failed"),
            "{err:?}"
        );
    }
}
