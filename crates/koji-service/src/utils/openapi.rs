//! Code-first OpenAPI document for the `/api/v2` surface.
//!
//! Generated from the v2 handlers (`#[utoipa::path(...)]`) and DTOs
//! (`#[derive(ToSchema)]`), registered in the `paths(...)` /
//! `components(schemas(...))` lists below. Served at `GET /api/v2/openapi.yaml`
//! by [`crate::openapi_spec`] — it replaces (and can't drift from) the
//! hand-maintained `openapi.yaml` that used to live beside the crate.
//!
//! `ApiDoc` is only referenced by `openapi_spec` (and the test below), so the
//! non-test build sees its associated items as dead — silenced crate-wide for
//! this module rather than item-by-item.
#![allow(dead_code)]

use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    info(
        title = "Koji v2 API",
        version = "2.0.0",
        description = "The /api/v2 surface of Koji — job-queue calc, typed CRUD, geometry utilities."
    ),
    paths(
        // jobs
        crate::public::v2::jobs::create_job,
        crate::public::v2::jobs::get_job,
        crate::public::v2::jobs::list_jobs,
        crate::public::v2::jobs::cancel_job,
        crate::public::v2::jobs::algorithms,
        // geofences
        crate::public::v2::geofences::list,
        crate::public::v2::geofences::create,
        crate::public::v2::geofences::get_one,
        crate::public::v2::geofences::update,
        crate::public::v2::geofences::remove,
        crate::public::v2::geofences::publish,
        // routes
        crate::public::v2::routes::list,
        crate::public::v2::routes::create,
        crate::public::v2::routes::get_one,
        crate::public::v2::routes::update,
        crate::public::v2::routes::remove,
        crate::public::v2::routes::publish,
        // typed CRUD resources (macro-generated)
        crate::public::v2::resources::project::list,
        crate::public::v2::resources::project::create,
        crate::public::v2::resources::project::get_one,
        crate::public::v2::resources::project::update,
        crate::public::v2::resources::project::remove,
        crate::public::v2::resources::property::list,
        crate::public::v2::resources::property::create,
        crate::public::v2::resources::property::get_one,
        crate::public::v2::resources::property::update,
        crate::public::v2::resources::property::remove,
        crate::public::v2::resources::tile_server::list,
        crate::public::v2::resources::tile_server::create,
        crate::public::v2::resources::tile_server::get_one,
        crate::public::v2::resources::tile_server::update,
        crate::public::v2::resources::tile_server::remove,
        // geometry
        crate::public::v2::geometry::convert,
        crate::public::v2::geometry::simplify,
        crate::public::v2::geometry::merge_points,
        crate::public::v2::geometry::calculate_area,
        // s2
        crate::public::v2::s2::circle_coverage,
        crate::public::v2::s2::cell_coverage,
        crate::public::v2::s2::cell_polygons,
        crate::public::v2::s2::s2_cells,
        // golbat-data
        crate::public::v2::golbat_data::golbat_data,
        crate::public::v2::golbat_data::by_area,
        crate::public::v2::golbat_data::area_stats,
        // plugins
        crate::public::v2::plugins::list,
        crate::public::v2::plugins::get_one,
        crate::public::v2::plugins::update,
        crate::public::v2::plugins::remove,
        // auth
        crate::public::v2::auth::login,
        crate::public::v2::auth::logout,
        crate::public::v2::auth::me,
        // config + nominatim
        crate::public::v2::config::config,
        crate::public::v2::nominatim::search_nominatim,
    ),
    components(schemas(
        // envelope
        crate::utils::api_response::ApiError,
        crate::utils::api_response::Meta,
        // calc request surface
        crate::requests::CalcJobRequest,
        crate::requests::CalcRequest,
        crate::requests::ClusterReq,
        crate::requests::RerouteReq,
        crate::requests::BootstrapReq,
        crate::requests::StatsReq,
        crate::requests::ConvertReq,
        crate::requests::SimplifyReq,
        crate::requests::MergePointsReq,
        crate::requests::ClusteringArgs,
        crate::requests::RoutingArgs,
        crate::requests::BootstrapArgs,
        crate::requests::DataFilterArgs,
        crate::requests::OutputArgs,
        crate::requests::DevArgs,
        crate::requests::ReturnTypeArg,
        // geofences / routes DTOs
        crate::public::v2::geofences::CreateGeofence,
        crate::public::v2::geofences::PatchGeofence,
        crate::public::v2::routes::CreateRoute,
        crate::public::v2::routes::PatchRoute,
        // typed CRUD resource DTOs (macro-generated)
        crate::public::v2::resources::project::CreateProject,
        crate::public::v2::resources::project::PatchProject,
        crate::public::v2::resources::property::CreateProperty,
        crate::public::v2::resources::property::PatchProperty,
        crate::public::v2::resources::tile_server::CreateTileServer,
        crate::public::v2::resources::tile_server::PatchTileServer,
        // s2 / golbat-data
        crate::public::v2::s2::CoverageArgs,
        crate::public::v2::s2::S2CellsBody,
        crate::public::v2::golbat_data::AreaReq,
        crate::public::v2::golbat_data::BboxInput,
        // config / plugins / auth
        crate::utils::response::ConfigResponse,
        crate::public::v2::plugins::PluginPatch,
        crate::private::Auth,
    )),
    tags(
        (name = "jobs", description = "Job-queue calc + algorithm metadata"),
        (name = "geofences", description = "Typed geofence CRUD + Dragonite publish"),
        (name = "routes", description = "Typed route CRUD + Dragonite publish"),
        (name = "projects", description = "Project CRUD"),
        (name = "properties", description = "Property CRUD"),
        (name = "tile-servers", description = "Tile-server CRUD"),
        (name = "geometry", description = "Geometry transforms (convert/simplify/merge-points/area)"),
        (name = "s2", description = "S2 cell helpers"),
        (name = "golbat-data", description = "Golbat data fetch (saved-fence + arbitrary-area)"),
        (name = "plugins", description = "DB-managed plugin config overlay"),
        (name = "auth", description = "Session auth (login/logout/me)"),
        (name = "config", description = "App bootstrap blob"),
        (name = "nominatim", description = "Nominatim geocoding proxy"),
    ),
)]
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

    /// The generated doc covers the core of the v2 surface: a representative
    /// path from each major area + the `ApiError` error schema. Guards against a
    /// handler silently dropping out of the `paths(...)` registration.
    #[test]
    fn openapi_covers_core_surface() {
        let doc = ApiDoc::openapi();
        let paths = doc.paths.paths.keys().cloned().collect::<Vec<_>>();
        for p in [
            "/api/v2/jobs",
            "/api/v2/jobs/{id}",
            "/api/v2/algorithms",
            "/api/v2/geofences",
            "/api/v2/geofences/{id}",
            "/api/v2/routes",
            "/api/v2/projects",
            "/api/v2/geometry/convert",
            "/api/v2/geometry/area",
            "/api/v2/s2/{cell_level}",
            "/api/v2/golbat-data/{category}",
            "/api/v2/plugins",
            "/api/v2/auth/login",
            "/api/v2/config",
            "/api/v2/nominatim",
        ] {
            assert!(paths.iter().any(|k| k == p), "missing path {p}");
        }

        // The shared error object is registered as a component schema.
        let schemas = doc
            .components
            .as_ref()
            .expect("components present")
            .schemas
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        assert!(
            schemas.iter().any(|k| k == "ApiError"),
            "ApiError schema missing from components"
        );
    }
}
