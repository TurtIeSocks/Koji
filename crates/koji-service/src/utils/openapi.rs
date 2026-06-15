//! Code-first OpenAPI document. Per-phase work annotates handlers with
//! `#[utoipa::path(...)]` and DTOs with `#[derive(ToSchema)]`, then registers
//! them in the `paths(...)` / `components(...)` lists below. This is the scaffold
//! (title + version) those phases extend; it is served at
//! `GET /api/v2/openapi.yaml`, replacing the hand-maintained `openapi.yaml`.
//!
//! Phase 0 lands the scaffold ahead of the route that serves it (wired in a
//! later phase), so `ApiDoc` is exercised only by the unit test below — silenced
//! crate-wide for this module rather than item-by-item.
#![allow(dead_code)]

use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(info(
    title = "Koji v2 API",
    version = "2.0.0",
    description = "The /api/v2 surface of Koji — job-queue calc, typed CRUD, geometry utilities."
))]
pub(crate) struct ApiDoc;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn openapi_builds_with_title_and_version() {
        let doc = ApiDoc::openapi();
        assert_eq!(doc.info.title, "Koji v2 API");
        assert_eq!(doc.info.version, "2.0.0");
    }
}
