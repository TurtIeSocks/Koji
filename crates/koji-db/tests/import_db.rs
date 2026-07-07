//! DB-gated integration for the atomic import. Gated on `KOJI_DB_URL`.
#![cfg(test)]

use koji_db::db::import::{ImportAction, ImportItem, ImportKind, OnCollision, import};
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
    let Some(db) = conn().await else {
        eprintln!("skip: KOJI_DB_URL unset");
        return;
    };
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
    let Some(db) = conn().await else {
        eprintln!("skip: KOJI_DB_URL unset");
        return;
    };
    let r = import(&db, vec![fence("itest-dryrun-only")], true)
        .await
        .unwrap();
    assert!(!r.committed);
    assert_eq!(r.results[0].action, ImportAction::Create);
}

#[tokio::test]
async fn rollback_on_unresolvable_route_parent_leaves_nothing() {
    let Some(db) = conn().await else {
        eprintln!("skip: KOJI_DB_URL unset");
        return;
    };
    // A valid fence + a route whose parent does not exist → validation fails the
    // route at dry-run, so commit is refused (committed:false), fence NOT written.
    let mut route = fence("itest-orphan-route");
    route.kind = ImportKind::Route;
    route.geometry = serde_json::json!({ "type": "MultiPoint", "coordinates": [[0.0,0.0]] });
    route.route_parent = Some("itest-nonexistent-parent".to_string());
    let r = import(&db, vec![fence("itest-rollback-fence"), route], false)
        .await
        .unwrap();
    assert!(!r.committed);
    assert!(r.results.iter().any(|o| o.action == ImportAction::Fail));
}

#[tokio::test]
async fn rollback_after_begin_leaves_nothing_written() {
    use koji_db::db::geofence;
    let Some(db) = conn().await else {
        eprintln!("skip: KOJI_DB_URL unset");
        return;
    };
    let name = "itest-rollback-mid-tx";

    // Validation passes (project ids are not pre-checked), but committing a
    // geofence_project row for a non-existent project violates the FK AFTER
    // db.begin() — exercising the post-begin rollback path (the route-parent
    // test fails at validation, before begin, so it does NOT cover this). The
    // whole tx must roll back, leaving the geofence UNwritten.
    let mut g = fence(name);
    g.projects = vec![999_999_999];

    let r = import(&db, vec![g], false).await.unwrap();
    assert!(
        !r.committed,
        "FK violation mid-tx must abort the commit: {:?}",
        r.results
    );
    assert!(
        r.results.iter().any(|o| o.action == ImportAction::Fail),
        "the offending item must be reported as a failure"
    );
    // Atomicity: the geofence inserted earlier in the same tx must be gone.
    assert!(
        geofence::Query::get_one(&db, name.to_string())
            .await
            .is_err(),
        "a rolled-back geofence must not be persisted"
    );
}

#[tokio::test]
async fn geofence_parent_forward_reference_links() {
    use koji_db::db::geofence;
    let Some(db) = conn().await else {
        eprintln!("skip: KOJI_DB_URL unset");
        return;
    };
    let parent_name = "itest-parent-fwd";
    let child_name = "itest-child-fwd";

    // Child lists its parent by name, and the child appears BEFORE the parent in
    // the batch — a single-pass import would resolve the parent to None and drop
    // the link silently. Overwrite so the assertion holds on repeated runs.
    let mut child = fence(child_name);
    child.parent = Some(parent_name.to_string());
    child.on_collision = OnCollision::Overwrite;
    let mut parent = fence(parent_name);
    parent.on_collision = OnCollision::Overwrite;

    let r = import(&db, vec![child, parent], false).await.unwrap();
    assert!(r.committed, "import should commit: {:?}", r.results);

    let parent_model = geofence::Query::get_one(&db, parent_name.to_string())
        .await
        .unwrap();
    let child_model = geofence::Query::get_one(&db, child_name.to_string())
        .await
        .unwrap();
    assert_eq!(
        child_model.parent,
        Some(parent_model.id),
        "child's parent must link the fence listed later in the batch"
    );
}
