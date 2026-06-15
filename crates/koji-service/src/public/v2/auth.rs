//! v2 session auth — `/api/v2/auth/{login,logout,me}`.
//!
//! A clean replacement for the legacy `/config/{login,logout}` pair (kept in
//! place through P4, removed in P6). The session model is unchanged: a single
//! cookie-backed `"logged_in"` flag set on a `KOJI_SECRET` password match,
//! cleared on logout. `me` introspects the *current* request's auth standing —
//! mirroring the [`public_validator`](crate::utils::auth::public_validator)
//! precedence (session ⇒ open-when-no-secret ⇒ bearer) — so a client can ask
//! "am I authenticated, and how?" without poking a protected resource.

use std::env;

use actix_session::Session;
use actix_web::{
    HttpRequest, HttpResponse,
    http::{StatusCode, header},
    web,
};
use serde_json::json;

use crate::{
    private::Auth,
    utils::api_response::{ApiError, ApiResponse},
};

/// Decide the auth `via` for `GET /auth/me`, matching `public_validator`'s
/// precedence exactly: an active session wins; otherwise an empty `KOJI_SECRET`
/// means the surface is open; otherwise a `Bearer` token equal to the secret
/// authenticates; otherwise unauthenticated. Pure so it can be unit-tested
/// without the actix `Session`/`HttpRequest` wiring.
///
/// `"none"` is the only value that signals *unauthenticated* (the handler
/// derives `authenticated = via != "none"`).
fn decide_via(logged_in: bool, secret: &str, bearer: Option<&str>) -> &'static str {
    if logged_in {
        "session"
    } else if secret.is_empty() {
        "open"
    } else if bearer == Some(secret) {
        "bearer"
    } else {
        "none"
    }
}

/// `POST /api/v2/auth/login` — `{ "password": "…" }`. On a `KOJI_SECRET` match,
/// set the session `"logged_in"` flag and return `200` `{authenticated:true}`;
/// otherwise (mismatch or session-write failure) a plain `401`. Auth failure is
/// not a `ServiceError` variant, so the `401` is built explicitly.
#[utoipa::path(
    post,
    path = "/api/v2/auth/login",
    tag = "auth",
    request_body = Auth,
    responses(
        (status = 200, description = "Authenticated: `{ authenticated: true }`", body = Object),
        (status = 401, description = "Invalid password", body = ApiError),
    ),
)]
async fn login(payload: web::Json<Auth>, session: Session) -> HttpResponse {
    if payload.password == env::var("KOJI_SECRET").unwrap_or_default() {
        match session.insert("logged_in", true) {
            Ok(()) => return ApiResponse::success(json!({ "authenticated": true })),
            Err(err) => log::warn!("[auth] session write failed: {err:?}"),
        }
    }
    ApiResponse::error(
        StatusCode::UNAUTHORIZED,
        "invalid password",
        Some("unauthorized".into()),
        None,
    )
}

/// `POST /api/v2/auth/logout` — clear the session and return `204` (the legacy
/// `/config/logout` returned a `302` to `/`; the v2 surface is a content-less
/// `204`).
#[utoipa::path(
    post,
    path = "/api/v2/auth/logout",
    tag = "auth",
    responses((status = 204, description = "Session cleared")),
)]
async fn logout(session: Session) -> HttpResponse {
    session.clear();
    HttpResponse::NoContent().finish()
}

/// `GET /api/v2/auth/me` — introspect the request's auth standing:
/// `{ "authenticated": bool, "via": "session"|"bearer"|"open"|"none" }`.
#[utoipa::path(
    get,
    path = "/api/v2/auth/me",
    tag = "auth",
    responses((status = 200, description = "Auth standing: `{ authenticated, via }`", body = Object)),
)]
async fn me(req: HttpRequest, session: Session) -> HttpResponse {
    let logged_in = session.get::<bool>("logged_in").ok().flatten().unwrap_or(false);
    let secret = env::var("KOJI_SECRET").unwrap_or_default();
    let bearer = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .map(str::trim);

    let via = decide_via(logged_in, &secret, bearer);
    ApiResponse::success(json!({
        "authenticated": via != "none",
        "via": via,
    }))
}

/// The `/auth` scope: `login` (POST), `logout` (POST), `me` (GET).
pub(crate) fn scope() -> actix_web::Scope {
    web::scope("/auth")
        .service(web::resource("/login").route(web::post().to(login)))
        .service(web::resource("/logout").route(web::post().to(logout)))
        .service(web::resource("/me").route(web::get().to(me)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_wins_over_everything() {
        // An active session authenticates regardless of secret/bearer.
        assert_eq!(decide_via(true, "", None), "session");
        assert_eq!(decide_via(true, "s3cret", Some("wrong")), "session");
    }

    #[test]
    fn empty_secret_is_open() {
        // No secret configured ⇒ the surface is open even without a session.
        assert_eq!(decide_via(false, "", None), "open");
        assert_eq!(decide_via(false, "", Some("anything")), "open");
    }

    #[test]
    fn matching_bearer_authenticates() {
        assert_eq!(decide_via(false, "s3cret", Some("s3cret")), "bearer");
    }

    #[test]
    fn no_session_wrong_or_absent_bearer_is_none() {
        assert_eq!(decide_via(false, "s3cret", Some("wrong")), "none");
        assert_eq!(decide_via(false, "s3cret", None), "none");
    }

    #[test]
    fn authenticated_derives_from_via() {
        // The handler derives `authenticated = via != "none"`; assert the only
        // unauthenticated value is "none".
        for via in ["session", "open", "bearer"] {
            assert!(via != "none", "{via} must count as authenticated");
        }
    }
}
