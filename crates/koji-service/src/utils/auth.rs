use std::env;

use actix_session::SessionExt;
use actix_web::dev::ServiceRequest;
use actix_web_httpauth::extractors::AuthExtractorConfig;

use actix_web_httpauth::extractors::{
    AuthenticationError,
    bearer::{BearerAuth, Config},
};
use subtle::ConstantTimeEq;

/// Constant-time string equality — avoids leaking the secret via comparison timing.
/// Different-length inputs compare unequal (length is not the secret here).
pub(crate) fn ct_eq(a: &str, b: &str) -> bool {
    a.as_bytes().ct_eq(b.as_bytes()).into()
}

fn logged_in(req: &ServiceRequest) -> bool {
    let session = req.get_session();
    if let Ok(logged_in) = session.get::<bool>("logged_in") {
        logged_in.unwrap_or(false)
    } else {
        false
    }
}

pub(crate) async fn public_validator(
    req: ServiceRequest,
    credentials: Option<BearerAuth>,
) -> Result<ServiceRequest, (actix_web::Error, ServiceRequest)> {
    if logged_in(&req) {
        return Ok(req);
    }
    if env::var("KOJI_SECRET").unwrap_or("".to_string()).is_empty() {
        return Ok(req);
    }
    if let Some(credentials) = credentials
        && ct_eq(
            credentials.token(),
            &env::var("KOJI_SECRET").unwrap_or_default(),
        )
    {
        return Ok(req);
    }
    Err((
        AuthenticationError::new(
            req.app_data::<Config>()
                .cloned()
                .unwrap_or_default()
                .into_inner(),
        )
        .into(),
        req,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ct_eq_matches_equal_strings() {
        assert!(ct_eq("s3cret", "s3cret"));
    }

    #[test]
    fn ct_eq_rejects_differing_strings() {
        assert!(!ct_eq("s3cret", "wrong!"));
    }

    #[test]
    fn ct_eq_rejects_different_lengths() {
        assert!(!ct_eq("s3cret", "s3cretX"));
        assert!(!ct_eq("s3cret", "s3cre"));
    }

    #[test]
    fn ct_eq_empty_vs_empty_is_true() {
        assert!(ct_eq("", ""));
    }

    #[test]
    fn ct_eq_empty_vs_nonempty_is_false() {
        assert!(!ct_eq("", "x"));
        assert!(!ct_eq("x", ""));
    }
}
