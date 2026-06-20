//! DB-gated integration for the atomic import. Gated on `KOJI_DB_URL`.
#![cfg(test)]

use koji_db::db::import::{import, ImportItem, ImportKind, OnCollision, ImportAction};
use sea_orm::{Database, DatabaseConnection};

async fn conn() -> Option<DatabaseConnection> {
    let url = std::env::var("KOJI_DB_URL").ok()?;
    Database::connect(url).await.ok()
}

fn fence(name: &str) -> ImportItem {
    ImportItem {
        kind: ImportKind::Geofence,
        name: name.to_string(),
        geometry: serde_json::json!({ "type": "Polygon", "coordinates": [[[0.0,0.0],[1.0,0.0],[1.0,1.0],[0.0,0.0]]] }),
        mode: Some("pokemon".to_string()),
        parent: None,
        projects: vec![],
        route_parent: None,
        on_collision: OnCollision::Skip,
    }
}

#[tokio::test]
async fn import_creates_then_retry_is_idempotent() {
    let Some(db) = conn().await else { eprintln!("skip: KOJI_DB_URL unset"); return; };
    let name = "itest-fence-idem";
    let r1 = import(&db, vec![fence(name)], false).await.unwrap();
    assert!(r1.committed);
    assert_eq!(r1.summary.create + r1.summary.update + r1.summary.skip, 1);

    // Retry: name now collides; default on_collision=skip → reported skip, no dup.
    let r2 = import(&db, vec![fence(name)], false).await.unwrap();
    assert!(r2.committed);
    assert_eq!(r2.results[0].action, ImportAction::Skip);
}

#[tokio::test]
async fn dry_run_writes_nothing_and_predicts() {
    let Some(db) = conn().await else { eprintln!("skip: KOJI_DB_URL unset"); return; };
    let r = import(&db, vec![fence("itest-dryrun-only")], true).await.unwrap();
    assert!(!r.committed);
    assert_eq!(r.results[0].action, ImportAction::Create);
}

#[tokio::test]
async fn rollback_on_unresolvable_route_parent_leaves_nothing() {
    let Some(db) = conn().await else { eprintln!("skip: KOJI_DB_URL unset"); return; };
    // A valid fence + a route whose parent does not exist → validation fails the
    // route at dry-run, so commit is refused (committed:false), fence NOT written.
    let mut route = fence("itest-orphan-route");
    route.kind = ImportKind::Route;
    route.geometry = serde_json::json!({ "type": "MultiPoint", "coordinates": [[0.0,0.0]] });
    route.route_parent = Some("itest-nonexistent-parent".to_string());
    let r = import(&db, vec![fence("itest-rollback-fence"), route], false).await.unwrap();
    assert!(!r.committed);
    assert!(r.results.iter().any(|o| o.action == ImportAction::Fail));
}
