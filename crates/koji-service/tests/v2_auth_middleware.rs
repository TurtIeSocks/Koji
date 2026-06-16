//! Integration tests for `utils/auth.rs` — the `public_validator` middleware.
// The `serial_guard()` pattern intentionally holds a `MutexGuard` across `.await`
// points to serialise concurrent test execution — this is safe in a single-threaded
// test context and is not a production-code hazard.
#![allow(clippy::await_holding_lock)]
//!
//! The validator allows a request if:
//!   1. The session has `logged_in = true`, OR
//!   2. `KOJI_SECRET` is unset or empty, OR
//!   3. The `Authorization: Bearer <token>` matches `KOJI_SECRET`.
//!
//! Otherwise it returns `401 Unauthorized`.
//!
//! These tests build a minimal actix App with `HttpAuthentication::with_fn(…)` on
//! a scope that exposes a trivial `/ping` endpoint, then drive it with
//! `test::TestRequest` to assert allow vs reject.
//!
//! No DB required. All tests that mutate `KOJI_SECRET` must hold `SERIAL.lock()`
//! for their entire body, since the env is shared across all test threads in this
//! binary.

use actix_session::{SessionMiddleware, storage::CookieSessionStore};
use actix_web::{App, HttpResponse, cookie::Key, get, test, web};
use actix_web_httpauth::middleware::HttpAuthentication;
use std::sync::{Mutex, MutexGuard};

/// Process-wide serialization so concurrent tests don't race on `KOJI_SECRET`.
static SERIAL: Mutex<()> = Mutex::new(());

fn serial_guard() -> MutexGuard<'static, ()> {
    SERIAL.lock().unwrap_or_else(|p| p.into_inner())
}

/// Thin handler the tests hit — always returns `200 ok` if auth passes.
#[get("/ping")]
async fn ping() -> HttpResponse {
    HttpResponse::Ok().body("pong")
}

/// A guard that resets `KOJI_SECRET` to its original value (or removes it)
/// when dropped, so a test that panics still cleans up.
struct SecretGuard(Option<String>);
impl SecretGuard {
    /// Snapshot the current value of `KOJI_SECRET` and set it to `new_value`.
    fn set(new_value: &str) -> Self {
        let original = std::env::var("KOJI_SECRET").ok();
        // SAFETY: single-threaded actix_web::test runtime; each test is isolated.
        unsafe {
            std::env::set_var("KOJI_SECRET", new_value);
        }
        Self(original)
    }
}
impl Drop for SecretGuard {
    fn drop(&mut self) {
        // SAFETY: same as above.
        unsafe {
            match &self.0 {
                Some(v) => std::env::set_var("KOJI_SECRET", v),
                None => std::env::remove_var("KOJI_SECRET"),
            }
        }
    }
}

/// Build a test app: the `/ping` route wrapped in `public_validator` auth.
/// The session middleware is required because `logged_in()` reads the session.
macro_rules! auth_app {
    () => {{
        test::init_service(
            App::new()
                .wrap(
                    SessionMiddleware::builder(CookieSessionStore::default(), Key::from(&[0; 64]))
                        .cookie_secure(false)
                        .build(),
                )
                .service(
                    web::scope("/api/v2")
                        .wrap(HttpAuthentication::with_fn(
                            koji_service::test_public_validator,
                        ))
                        .service(ping),
                ),
        )
        .await
    }};
}

// ── no-secret (KOJI_SECRET unset) → always allowed ──────────────────────────

#[actix_web::test]
async fn no_secret_allows_unauthenticated_request() {
    let _serial = serial_guard();
    let _guard = SecretGuard::set(""); // empty → treated as "no secret"
    let app = auth_app!();
    let req = test::TestRequest::get().uri("/api/v2/ping").to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
}

#[actix_web::test]
async fn no_secret_env_var_unset_allows_any_bearer() {
    let _serial = serial_guard();
    let _guard = SecretGuard::set("");
    let app = auth_app!();
    let req = test::TestRequest::get()
        .uri("/api/v2/ping")
        .insert_header(("Authorization", "Bearer some-random-token"))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200, "no secret → any bearer allowed");
}

// ── KOJI_SECRET set → bearer must match ────────────────────────────────────

#[actix_web::test]
async fn correct_bearer_token_is_allowed() {
    let _serial = serial_guard();
    let _guard = SecretGuard::set("super-secret-token");
    let app = auth_app!();
    let req = test::TestRequest::get()
        .uri("/api/v2/ping")
        .insert_header(("Authorization", "Bearer super-secret-token"))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200, "correct bearer must be allowed");
}

#[actix_web::test]
async fn wrong_bearer_token_is_rejected() {
    let _serial = serial_guard();
    let _guard = SecretGuard::set("super-secret-token");
    let app = auth_app!();
    let req = test::TestRequest::get()
        .uri("/api/v2/ping")
        .insert_header(("Authorization", "Bearer wrong-token"))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401, "wrong bearer must be rejected");
}

#[actix_web::test]
async fn missing_bearer_header_is_rejected_when_secret_set() {
    let _serial = serial_guard();
    let _guard = SecretGuard::set("super-secret-token");
    let app = auth_app!();
    let req = test::TestRequest::get().uri("/api/v2/ping").to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401, "no bearer at all must be rejected");
}

#[actix_web::test]
async fn empty_bearer_token_is_rejected_when_secret_set() {
    let _serial = serial_guard();
    let _guard = SecretGuard::set("super-secret-token");
    let app = auth_app!();
    // "Bearer " with nothing after it
    let req = test::TestRequest::get()
        .uri("/api/v2/ping")
        .insert_header(("Authorization", "Bearer "))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401, "empty bearer must be rejected");
}

// ── case-sensitivity ────────────────────────────────────────────────────────

#[actix_web::test]
async fn bearer_token_comparison_is_case_sensitive() {
    let _serial = serial_guard();
    let _guard = SecretGuard::set("CaseSensitiveSecret");
    let app = auth_app!();
    let req = test::TestRequest::get()
        .uri("/api/v2/ping")
        .insert_header(("Authorization", "Bearer casesensitivesecret"))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 401, "token comparison is case-sensitive");
}

// ── lib.rs re-export used by auth_app! ──────────────────────────────────────
// `test_public_validator` is exposed from `lib.rs` as a thin re-export of the
// `pub(crate)` `utils::auth::public_validator` — see the comment there.
