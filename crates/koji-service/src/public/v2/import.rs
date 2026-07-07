//! `POST /internal/import` — atomic bulk import. Maps the HTTP DTO to the
//! koji-db `import` orchestrator (one tx; dry-run = validate-only preview).

use std::collections::{BTreeSet, HashMap, HashSet};

use actix_web::{HttpResponse, web};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use koji_db::KojiDb;
use koji_db::db::geofence_project;
use koji_db::db::import as dbimport;

use crate::utils::api_response::ApiResponse;
use crate::utils::error::ServiceError;

#[derive(Debug, Clone, Copy, Deserialize, Serialize, ToSchema, Default)]
#[serde(rename_all = "lowercase")]
pub(crate) enum ItemKind {
    #[default]
    Geofence,
    Route,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, ToSchema, Default)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Collision {
    #[default]
    Skip,
    Overwrite,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub(crate) struct ImportItemDto {
    #[serde(default)]
    pub kind: ItemKind,
    pub name: String,
    #[schema(value_type = Object)]
    pub geometry: serde_json::Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
    #[serde(default)]
    pub projects: Vec<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub route_parent: Option<String>,
    #[serde(default)]
    pub on_collision: Collision,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub(crate) struct ImportRequest {
    #[serde(default)]
    pub dry_run: bool,
    pub items: Vec<ImportItemDto>,
}

pub(crate) fn to_db_items(req: &ImportRequest) -> Vec<dbimport::ImportItem> {
    req.items
        .iter()
        .map(|it| dbimport::ImportItem {
            kind: match it.kind {
                ItemKind::Geofence => dbimport::ImportKind::Geofence,
                ItemKind::Route => dbimport::ImportKind::Route,
            },
            name: it.name.clone(),
            geometry: it.geometry.clone(),
            mode: it.mode.clone(),
            parent: it.parent.clone(),
            projects: it.projects.clone(),
            route_parent: it.route_parent.clone(),
            on_collision: match it.on_collision {
                Collision::Skip => dbimport::OnCollision::Skip,
                Collision::Overwrite => dbimport::OnCollision::Overwrite,
            },
        })
        .collect()
}

fn result_to_json(r: &dbimport::ImportResult) -> serde_json::Value {
    let action = |a: dbimport::ImportAction| match a {
        dbimport::ImportAction::Create => "create",
        dbimport::ImportAction::Update => "update",
        dbimport::ImportAction::Skip => "skip",
        dbimport::ImportAction::Fail => "fail",
    };
    serde_json::json!({
        "committed": r.committed,
        "summary": {
            "create": r.summary.create,
            "update": r.summary.update,
            "skip": r.summary.skip,
            "fail": r.summary.fail,
        },
        "results": r.results.iter().map(|o| serde_json::json!({
            "index": o.index,
            "name": o.name,
            "action": action(o.action),
            "id": o.id,
            "reason": o.reason,
        })).collect::<Vec<_>>(),
    })
}

/// `POST /internal/import` — atomic bulk import (dry-run preview when `dry_run`).
///
/// Membership-diff emission (`project.geofences_changed`, Task 8): the import
/// tx lives entirely inside `dbimport::import` (one `txn.begin()`/`commit()`),
/// so this handler snapshots each mentioned project's linked-fence set
/// *before* calling it, then — only if the import actually committed —
/// re-resolves the same projects *after* and diffs. Snapshot-before/resolve-
/// after (not accumulate-in-loop) because an `Overwrite` collision replaces an
/// existing fence's project links wholesale (`geofence_project`'s
/// `upsert_related_*` deletes stale rows), so accumulate-only would miss
/// removals. Emitting only when `result.committed` is true keeps a
/// rolled-back/dry-run import from lying about a membership change that never
/// happened.
pub(crate) async fn import_handler(
    conn: web::Data<KojiDb>,
    body: web::Json<ImportRequest>,
) -> Result<HttpResponse, ServiceError> {
    let req = body.into_inner();
    let dry_run = req.dry_run;
    let items = to_db_items(&req);

    // Every project id any geofence item in this batch mentions.
    let touched_projects: HashSet<u32> = req
        .items
        .iter()
        .flat_map(|it| it.projects.iter().copied())
        .collect();

    let before = snapshot_project_membership(&conn.koji, &touched_projects).await;

    let result = dbimport::import(&conn.koji, items, dry_run)
        .await
        .map_err(ServiceError::internal)?;

    if result.committed {
        let after = snapshot_project_membership(&conn.koji, &touched_projects).await;
        crate::utils::outbox::emit_membership_diff(&conn.koji, &before, &after).await;
    }

    Ok(ApiResponse::success(result_to_json(&result)))
}

/// Resolve `project_id -> {linked geofence ids}` for a set of projects, for
/// before/after membership-diff snapshots around the import tx.
async fn snapshot_project_membership(
    db: &sea_orm::DatabaseConnection,
    project_ids: &HashSet<u32>,
) -> HashMap<u32, BTreeSet<u32>> {
    let mut map = HashMap::new();
    for &pid in project_ids {
        let ids = geofence_project::Query::geofence_ids_for_project(db, pid)
            .await
            .unwrap_or_default();
        map.insert(pid, ids.into_iter().collect());
    }
    map
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_request_items_to_db_items_with_defaults() {
        let req: ImportRequest = serde_json::from_value(serde_json::json!({
            "dry_run": true,
            "items": [
                { "kind": "geofence", "name": "A", "geometry": { "type": "Polygon", "coordinates": [] } },
                { "kind": "route", "name": "r", "geometry": { "type": "MultiPoint", "coordinates": [] },
                  "route_parent": "A", "on_collision": "overwrite", "projects": [1, 2] }
            ]
        }))
        .unwrap();
        let items = to_db_items(&req);
        assert_eq!(items.len(), 2);
        // defaults: missing on_collision -> Skip, missing projects -> empty
        assert!(matches!(items[0].kind, dbimport::ImportKind::Geofence));
        assert!(matches!(items[0].on_collision, dbimport::OnCollision::Skip));
        assert!(items[0].projects.is_empty());
        assert!(matches!(items[1].kind, dbimport::ImportKind::Route));
        assert!(matches!(items[1].on_collision, dbimport::OnCollision::Overwrite));
        assert_eq!(items[1].route_parent.as_deref(), Some("A"));
        assert_eq!(items[1].projects, vec![1, 2]);
    }

    #[actix_web::test]
    async fn internal_import_route_is_registered() {
        use actix_web::{App, test};
        let app = test::init_service(App::new().service(crate::internal::scope())).await;
        // No auth middleware here -> the handler is reached; an empty/invalid body
        // fails the Json extractor (400), proving the route exists (NOT 404).
        let req = test::TestRequest::post()
            .uri("/internal/import")
            .set_json(serde_json::json!({}))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_ne!(
            resp.status().as_u16(),
            404,
            "POST /internal/import must be routed"
        );
    }
}
