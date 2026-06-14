//! Macro-driven CRUD for the v2 plain-JSON resources (`projects`, `properties`,
//! `tile-servers`).
//!
//! These three resources share the exact same koji-db `Query` surface —
//! `get_json_cache` / `get_one_json` / `upsert_json_return` / `delete` — and are
//! not geometry-bearing, so they don't need the `?format=` return-type handling
//! that geofences/routes use. The [`crud_resource!`] macro stamps out the five
//! REST handlers (list / create / get-one / update / delete) plus a
//! [`actix_web::Scope`] builder per resource from the koji-db `Query` type + URL
//! path segment, wrapping every result in the
//! [`ApiResponse`](crate::utils::api_response::ApiResponse) envelope.
//!
//! The handlers are wired with `web::resource(..).route(..)` rather than the
//! `#[get]`/`#[post]` attribute macros because attribute macros require a string
//! *literal* path — they can't take the macro's `$seg` token — so each resource
//! exposes a `scope()` that `crate::start` mounts into `/api/v2`.
//!
//! Geofences and routes are hand-written (see [`super::geofences`] /
//! [`super::routes`]) because their koji-db signatures genuinely differ (they
//! carry geometry and take `ApiQueryArgs` / an `internal` flag).

use actix_web::{Error, HttpResponse, http::StatusCode, web};
use koji_db::KojiDb;
use serde_json::json;

use crate::utils::api_response::ApiResponse;

/// Stamp out the five CRUD handlers + a [`Scope`](actix_web::Scope) builder for a
/// plain-JSON resource.
///
/// `$module` is the koji-db `db::<module>::Query` to call; `$seg` is the URL
/// path segment (e.g. `"projects"`). Everything is emitted into a
/// `$module`-named submodule so handler names don't collide across resources;
/// `crate::start` mounts each via `resources::project::scope()` etc.
macro_rules! crud_resource {
    ($module:ident, $seg:literal) => {
        /// Generated CRUD handlers + scope for the `$seg` resource. See
        /// [`crud_resource!`](crate::public::v2::resources).
        pub(crate) mod $module {
            use super::*;
            use koji_db::db::$module::Query;

            /// `GET /api/v2/$seg` — list all records (plain ApiResponse array).
            pub(crate) async fn list(db: web::Data<KojiDb>) -> Result<HttpResponse, Error> {
                let rows = Query::get_json_cache(&db.koji)
                    .await
                    .map_err(actix_web::error::ErrorInternalServerError)?;
                Ok(ApiResponse::success(rows))
            }

            /// `POST /api/v2/$seg` — create a record → `201` ApiResponse.
            pub(crate) async fn create(
                db: web::Data<KojiDb>,
                payload: web::Json<serde_json::Value>,
            ) -> Result<HttpResponse, Error> {
                let record = Query::upsert_json_return(&db.koji, 0, payload.into_inner())
                    .await
                    .map_err(actix_web::error::ErrorInternalServerError)?;
                Ok(ApiResponse::success_with_status(StatusCode::CREATED, record))
            }

            /// `GET /api/v2/$seg/{id}` — fetch one record by id or name.
            pub(crate) async fn get_one(
                db: web::Data<KojiDb>,
                path: web::Path<String>,
            ) -> Result<HttpResponse, Error> {
                let record = Query::get_one_json(&db.koji, path.into_inner())
                    .await
                    .map_err(actix_web::error::ErrorInternalServerError)?;
                Ok(ApiResponse::success(record))
            }

            /// `PATCH /api/v2/$seg/{id}` — update a record by id.
            pub(crate) async fn update(
                db: web::Data<KojiDb>,
                path: web::Path<u32>,
                payload: web::Json<serde_json::Value>,
            ) -> Result<HttpResponse, Error> {
                let record =
                    Query::upsert_json_return(&db.koji, path.into_inner(), payload.into_inner())
                        .await
                        .map_err(actix_web::error::ErrorInternalServerError)?;
                Ok(ApiResponse::success(record))
            }

            /// `DELETE /api/v2/$seg/{id}` — delete a record → ApiResponse `{rows_affected}`.
            pub(crate) async fn remove(
                db: web::Data<KojiDb>,
                path: web::Path<u32>,
            ) -> Result<HttpResponse, Error> {
                let result = Query::delete(&db.koji, path.into_inner())
                    .await
                    .map_err(actix_web::error::ErrorInternalServerError)?;
                Ok(ApiResponse::success(json!({ "rows_affected": result.rows_affected })))
            }

            /// The `web::Scope` wiring the five handlers under `/$seg`, to be
            /// mounted into `/api/v2` by [`crate::start`].
            pub(crate) fn scope() -> actix_web::Scope {
                web::scope(concat!("/", $seg))
                    .service(
                        web::resource("")
                            .route(web::get().to(list))
                            .route(web::post().to(create)),
                    )
                    .service(
                        web::resource("/{id}")
                            .route(web::get().to(get_one))
                            .route(web::patch().to(update))
                            .route(web::delete().to(remove)),
                    )
            }
        }
    };
}

crud_resource!(project, "projects");
crud_resource!(property, "properties");
crud_resource!(tile_server, "tile-servers");
