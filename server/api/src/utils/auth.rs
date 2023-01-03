use std::env;

use actix_session::SessionExt;
use actix_web::dev::ServiceRequest;
use actix_web_httpauth::extractors::AuthExtractorConfig;

use actix_web_httpauth::extractors::{
    bearer::{BearerAuth, Config},
    AuthenticationError,
};

fn logged_in(req: &ServiceRequest) -> bool {
    let session = req.get_session();
    if let Ok(logged_in) = session.get::<bool>("logged_in") {
        logged_in.unwrap_or(false)
    } else {
        false
    }
}

pub async fn public_validator(
    req: ServiceRequest,
    credentials: Option<BearerAuth>,
) -> Result<ServiceRequest, (actix_web::Error, ServiceRequest)> {
    if logged_in(&req) {
        return Ok(req);
    }
    if let Some(credentials) = credentials {
        if credentials.token() == env::var("KOJI_SECRET").unwrap_or("".to_string()) {
            return Ok(req);
        }
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

pub async fn private_validator(
    req: ServiceRequest,
    _credentials: Option<BearerAuth>,
) -> Result<ServiceRequest, (actix_web::Error, ServiceRequest)> {
    if logged_in(&req) {
        Ok(req)
    } else {
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
}
