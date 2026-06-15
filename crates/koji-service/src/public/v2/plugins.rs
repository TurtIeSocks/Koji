//! v2 plugin management — `/api/v2/plugins` (architecture: DB-managed plugin
//! config). Reads the merged disk-manifest + DB-overlay view; PATCH/DELETE edit
//! only the overlay (`enabled`/`args_default`/`description`). The executable
//! declaration (`entrypoint`/`interpreter`/`protocol`) is disk-owned + read-only.
//!
//! Mounted under the shared `public_validator` like the other v2 resources; a
//! dedicated gate is deferred to the global API-security rework.

use actix_web::{Error, HttpResponse, web};
use koji_db::{KojiDb, db::plugin_config};
use koji_plugins::{PluginKind, PluginRegistry};
use serde::Deserialize;
use serde_json::{Value, json};
use utoipa::ToSchema;

use crate::utils::{
    api_response::{ApiError, ApiResponse},
    error::ServiceError,
};

/// Parse a `{kind}` path segment into a [`PluginKind`]. The plugin `name` is a
/// free-form segment carried alongside it (`/plugins/{kind}/{name}`), so only
/// the discriminator is validated here.
fn parse_kind(kind: &str) -> Option<PluginKind> {
    match kind {
        "clustering" => Some(PluginKind::Clustering),
        "routing" => Some(PluginKind::Routing),
        "bootstrap" => Some(PluginKind::Bootstrap),
        _ => None,
    }
}

/// PATCH body — every field optional (overlay merge).
#[derive(Debug, Deserialize, ToSchema)]
pub(crate) struct PluginPatch {
    enabled: Option<bool>,
    /// Default plugin CLI args, as opaque JSON.
    #[schema(value_type = Option<Object>)]
    args_default: Option<Value>,
    description: Option<String>,
}

/// Build the merged view (disk manifest + overlay) for one plugin from the
/// current registry, as the JSON the API returns. `None` when no disk manifest
/// exists for `(kind, name)` (the executable declaration is disk-owned, so an
/// overlay without a manifest is not a real plugin).
fn plugin_view(reg: &PluginRegistry, kind: PluginKind, name: &str) -> Option<Value> {
    let manifest = reg.manifest_unfiltered(kind, name)?;
    Some(json!({
        "id": format!("{kind}:{name}"),
        "name": name,
        "kind": kind.to_string(),
        "entrypoint": manifest.entrypoint,
        "interpreter": manifest.interpreter,
        "protocol": format!("{:?}", manifest.protocol).to_lowercase(),
        "version": manifest.version,
        "description": manifest.description,
        "enabled": reg.is_enabled(kind, name),
        "args_default": reg.args_default(kind, name),
    }))
}

/// Rebuild the registry from disk ∪ DB overlay and install it process-wide.
/// Called once at startup and after every overlay write so the runtime
/// dispatch path (and `meta/algorithms`) reflects the latest config.
pub(crate) async fn rebuild_and_install(db: &KojiDb) -> Result<(), Error> {
    let mut reg = PluginRegistry::from_env();
    let rows = plugin_config::Query::all(&db.koji)
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;
    for row in rows {
        if let Some(kind) = parse_kind(&row.kind) {
            reg.set_overlay(
                kind,
                &row.name,
                koji_plugins::Overlay {
                    enabled: row.enabled,
                    args_default: row.args_default,
                },
            );
        }
    }
    koji_plugins::install(reg);
    Ok(())
}

/// `GET /api/v2/plugins` — every disk-discovered plugin (enabled or not) merged
/// with its overlay.
#[utoipa::path(
    get,
    path = "/api/v2/plugins",
    tag = "plugins",
    responses((status = 200, description = "Every disk-discovered plugin merged with its overlay", body = Object)),
)]
async fn list(_db: web::Data<KojiDb>) -> Result<HttpResponse, ServiceError> {
    let reg = koji_plugins::current();
    let views: Vec<Value> = reg
        .all_unfiltered_keys()
        .into_iter()
        .filter_map(|(kind, name)| plugin_view(&reg, kind, &name))
        .collect();
    Ok(ApiResponse::success(views))
}

/// `GET /api/v2/plugins/{kind}/{name}` — one plugin's merged view; `404` for an
/// unknown `kind` or when no plugin matches.
#[utoipa::path(
    get,
    path = "/api/v2/plugins/{kind}/{name}",
    tag = "plugins",
    params(
        ("kind" = String, Path, description = "Plugin kind (`clustering|routing|bootstrap`)"),
        ("name" = String, Path, description = "Plugin name"),
    ),
    responses(
        (status = 200, description = "The plugin's merged view", body = Object),
        (status = 404, description = "Unknown kind or no such plugin", body = ApiError),
    ),
)]
async fn get_one(path: web::Path<(String, String)>) -> Result<HttpResponse, ServiceError> {
    let (kind, name) = path.into_inner();
    let Some(kind) = parse_kind(&kind) else {
        return Err(ServiceError::NotFound {
            field: "plugin",
            message: format!("no plugin {kind}/{name}"),
        });
    };
    let reg = koji_plugins::current();
    match plugin_view(&reg, kind, &name) {
        Some(view) => Ok(ApiResponse::success(view)),
        None => Err(ServiceError::NotFound {
            field: "plugin",
            message: format!("no plugin {kind}/{name}"),
        }),
    }
}

/// `PATCH /api/v2/plugins/{kind}/{name}` — upsert the overlay (`enabled`/
/// `args_default`/`description`) and rebuild the registry. `404` unknown `kind`,
/// `422` when no disk manifest exists for the id.
#[utoipa::path(
    patch,
    path = "/api/v2/plugins/{kind}/{name}",
    tag = "plugins",
    params(
        ("kind" = String, Path, description = "Plugin kind (`clustering|routing|bootstrap`)"),
        ("name" = String, Path, description = "Plugin name"),
    ),
    request_body = PluginPatch,
    responses(
        (status = 200, description = "The plugin's updated merged view", body = Object),
        (status = 404, description = "Unknown kind", body = ApiError),
        (status = 422, description = "No installed plugin for the id (drop a plugin.toml first)", body = ApiError),
    ),
)]
async fn update(
    db: web::Data<KojiDb>,
    path: web::Path<(String, String)>,
    body: web::Json<PluginPatch>,
) -> Result<HttpResponse, ServiceError> {
    let (kind, name) = path.into_inner();
    let Some(kind) = parse_kind(&kind) else {
        return Err(ServiceError::NotFound {
            field: "plugin",
            message: format!("no plugin {kind}/{name}"),
        });
    };
    // Enforce the disk gate: only configure a plugin that exists on disk.
    if koji_plugins::current()
        .manifest_unfiltered(kind, &name)
        .is_none()
    {
        return Err(ServiceError::Unprocessable {
            field: Some("id".into()),
            message: format!("no installed plugin {kind}/{name}; drop a plugin.toml first"),
        });
    }
    let patch = body.into_inner();
    plugin_config::Query::upsert(
        &db.koji,
        &kind.to_string(),
        &name,
        patch.enabled,
        patch.args_default,
        patch.description,
    )
    .await?;
    rebuild_and_install(&db)
        .await
        .map_err(ServiceError::internal)?;
    let reg = koji_plugins::current();
    Ok(ApiResponse::success(plugin_view(&reg, kind, &name)))
}

/// `DELETE /api/v2/plugins/{kind}/{name}` — drop the overlay (reset to disk
/// defaults) and rebuild the registry. `404` for an unknown `kind`.
#[utoipa::path(
    delete,
    path = "/api/v2/plugins/{kind}/{name}",
    tag = "plugins",
    params(
        ("kind" = String, Path, description = "Plugin kind (`clustering|routing|bootstrap`)"),
        ("name" = String, Path, description = "Plugin name"),
    ),
    responses(
        (status = 200, description = "Overlay dropped: `{ rows_affected }`", body = Object),
        (status = 404, description = "Unknown kind", body = ApiError),
    ),
)]
async fn remove(
    db: web::Data<KojiDb>,
    path: web::Path<(String, String)>,
) -> Result<HttpResponse, ServiceError> {
    let (kind, name) = path.into_inner();
    let Some(kind) = parse_kind(&kind) else {
        return Err(ServiceError::NotFound {
            field: "plugin",
            message: format!("no plugin {kind}/{name}"),
        });
    };
    let result = plugin_config::Query::delete(&db.koji, &kind.to_string(), &name).await?;
    rebuild_and_install(&db)
        .await
        .map_err(ServiceError::internal)?;
    Ok(ApiResponse::success(
        json!({ "rows_affected": result.rows_affected }),
    ))
}

/// The `/plugins` scope: collection `GET`, item `GET`/`PATCH`/`DELETE` keyed by
/// `/{kind}/{name}` path segments.
pub(crate) fn scope() -> actix_web::Scope {
    web::scope("/plugins")
        .service(web::resource("").route(web::get().to(list)))
        .service(
            web::resource("/{kind}/{name}")
                .route(web::get().to(get_one))
                .route(web::patch().to(update))
                .route(web::delete().to(remove)),
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_kind_maps_known_discriminators() {
        assert_eq!(parse_kind("routing"), Some(PluginKind::Routing));
        assert_eq!(parse_kind("clustering"), Some(PluginKind::Clustering));
        assert_eq!(parse_kind("bootstrap"), Some(PluginKind::Bootstrap));
    }

    #[test]
    fn parse_kind_rejects_unknown_discriminator() {
        assert!(parse_kind("teleport").is_none());
        assert!(parse_kind("tsp").is_none());
        assert!(parse_kind("").is_none());
    }
}
