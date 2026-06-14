//! v2 plugin management — `/api/v2/plugins` (architecture: DB-managed plugin
//! config). Reads the merged disk-manifest + DB-overlay view; PATCH/DELETE edit
//! only the overlay (`enabled`/`args_default`/`description`). The executable
//! declaration (`entrypoint`/`interpreter`/`protocol`) is disk-owned + read-only.
//!
//! Mounted under the shared `public_validator` like the other v2 resources; a
//! dedicated gate is deferred to the global API-security rework.

use actix_web::{Error, HttpResponse, http::StatusCode, web};
use koji_db::{KojiDb, db::plugin_config};
use koji_plugins::{PluginKind, PluginRegistry};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::utils::api_response::ApiResponse;

/// Parse a `"{kind}:{name}"` resource id. `name` may itself contain `:`.
fn parse_id(id: &str) -> Option<(PluginKind, String)> {
    let (kind, name) = id.split_once(':')?;
    let kind = match kind {
        "clustering" => PluginKind::Clustering,
        "routing" => PluginKind::Routing,
        "bootstrap" => PluginKind::Bootstrap,
        _ => return None,
    };
    if name.is_empty() {
        return None;
    }
    Some((kind, name.to_string()))
}

/// PATCH body — every field optional (overlay merge).
#[derive(Debug, Deserialize)]
struct PluginPatch {
    enabled: Option<bool>,
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
        if let Some((kind, name)) = parse_id(&format!("{}:{}", row.kind, row.name)) {
            reg.set_overlay(
                kind,
                &name,
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
async fn list(_db: web::Data<KojiDb>) -> Result<HttpResponse, Error> {
    let reg = koji_plugins::current();
    let views: Vec<Value> = reg
        .all_unfiltered_keys()
        .into_iter()
        .filter_map(|(kind, name)| plugin_view(&reg, kind, &name))
        .collect();
    Ok(ApiResponse::success(views))
}

/// `GET /api/v2/plugins/{id}` — one plugin's merged view; `400` for a malformed
/// id, `404` when no plugin matches.
async fn get_one(path: web::Path<String>) -> Result<HttpResponse, Error> {
    let id = path.into_inner();
    let Some((kind, name)) = parse_id(&id) else {
        return Ok(ApiResponse::fail(
            StatusCode::BAD_REQUEST,
            json!({ "id": "expected {kind}:{name}" }),
        ));
    };
    let reg = koji_plugins::current();
    match plugin_view(&reg, kind, &name) {
        Some(view) => Ok(ApiResponse::success(view)),
        None => Ok(ApiResponse::fail(
            StatusCode::NOT_FOUND,
            json!({ "id": format!("no plugin {id}") }),
        )),
    }
}

/// `PATCH /api/v2/plugins/{id}` — upsert the overlay (`enabled`/`args_default`/
/// `description`) and rebuild the registry. `400` malformed id, `422` when no
/// disk manifest exists for the id.
async fn update(
    db: web::Data<KojiDb>,
    path: web::Path<String>,
    body: web::Json<PluginPatch>,
) -> Result<HttpResponse, Error> {
    let id = path.into_inner();
    let Some((kind, name)) = parse_id(&id) else {
        return Ok(ApiResponse::fail(
            StatusCode::BAD_REQUEST,
            json!({ "id": "expected {kind}:{name}" }),
        ));
    };
    // Enforce the disk gate: only configure a plugin that exists on disk.
    if koji_plugins::current()
        .manifest_unfiltered(kind, &name)
        .is_none()
    {
        return Ok(ApiResponse::fail(
            StatusCode::UNPROCESSABLE_ENTITY,
            json!({ "id": format!("no installed plugin {id}; drop a plugin.toml first") }),
        ));
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
    .await
    .map_err(actix_web::error::ErrorInternalServerError)?;
    rebuild_and_install(&db).await?;
    let reg = koji_plugins::current();
    Ok(ApiResponse::success(plugin_view(&reg, kind, &name)))
}

/// `DELETE /api/v2/plugins/{id}` — drop the overlay (reset to disk defaults) and
/// rebuild the registry. `400` for a malformed id.
async fn remove(db: web::Data<KojiDb>, path: web::Path<String>) -> Result<HttpResponse, Error> {
    let id = path.into_inner();
    let Some((kind, name)) = parse_id(&id) else {
        return Ok(ApiResponse::fail(
            StatusCode::BAD_REQUEST,
            json!({ "id": "expected {kind}:{name}" }),
        ));
    };
    let result = plugin_config::Query::delete(&db.koji, &kind.to_string(), &name)
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;
    rebuild_and_install(&db).await?;
    Ok(ApiResponse::success(
        json!({ "rows_affected": result.rows_affected }),
    ))
}

/// The `/plugins` scope: collection `GET`, item `GET`/`PATCH`/`DELETE`.
pub(crate) fn scope() -> actix_web::Scope {
    web::scope("/plugins")
        .service(web::resource("").route(web::get().to(list)))
        .service(
            web::resource("/{id}")
                .route(web::get().to(get_one))
                .route(web::patch().to(update))
                .route(web::delete().to(remove)),
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_id_splits_kind_and_name() {
        assert_eq!(
            parse_id("routing:tsp").unwrap(),
            (PluginKind::Routing, "tsp".to_string())
        );
        assert_eq!(
            parse_id("clustering:k:means").unwrap(),
            (PluginKind::Clustering, "k:means".to_string())
        );
    }

    #[test]
    fn parse_id_rejects_missing_colon_and_bad_kind() {
        assert!(parse_id("tsp").is_none());
        assert!(parse_id("teleport:x").is_none());
    }
}
