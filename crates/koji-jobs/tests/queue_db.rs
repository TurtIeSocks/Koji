//! DB-integration tests for the [`JobQueue`] lifecycle (design spec §5–§7, §10).
//!
//! These run against a **live** MySQL `koji_test` (already migrated). They are
//! the first DB-backed tests in the workspace — the crate's own unit tests are
//! deliberately DB-free (see the lib docs).
//!
//! ## Gating (no `#[ignore]`)
//! Every test starts with `let Some(db) = test_db().await else { return };`.
//! `test_db()` reads `KOJI_DB_URL` and connects; if the var is unset it prints a
//! skip line and returns `None`, so a plain `cargo test` (no env) **passes by
//! skipping**. With the env sourced, the tests actually exercise the DB:
//!
//! ```sh
//! set -a; source ./.env.test; set +a
//! cargo test -p koji-jobs --test queue_db -- --nocapture
//! ```
//!
//! ## Isolation
//! Each test scopes its rows by a unique `kind` derived from a fresh
//! [`JobId`] (ULID), so parallel + repeat runs never collide, and **deletes
//! every row it creates** at the end via a raw `DELETE ... WHERE kind = ?` so
//! `koji_test` stays clean and the tests are re-runnable.

use std::time::Duration;

use koji_jobs::{JobId, JobOutcome, JobQueue, JobStatus, dedup_key};
use sea_orm::{ConnectionTrait, Database, DatabaseConnection, DbBackend, Statement, Value};
use tokio::sync::{Mutex, MutexGuard};

/// Process-wide serialization for the queue tests.
///
/// `cargo test` runs the tests in one binary **in parallel**, but they all
/// contend on the single `job` table with `FOR UPDATE` / `FOR UPDATE SKIP
/// LOCKED` locks (`enqueue_or_attach` dedup set, `claim` candidate scan). Six
/// parallel transactions on overlapping index ranges deadlock (MySQL 1213) or
/// starve each other — none of which is what these tests are checking. Real
/// Koji runs one worker pool, so serializing here matches production and makes
/// the lifecycle assertions deterministic, with no extra dev-dependency.
static SERIAL: Mutex<()> = Mutex::const_new(());

/// Acquire the process-wide test lock; held for the test body's lifetime.
async fn serial_guard() -> MutexGuard<'static, ()> {
    SERIAL.lock().await
}

/// Connect to the koji DB iff `KOJI_DB_URL` is set. Returns `None` (after a skip
/// note) when the var is missing, so the suite passes by skipping with no env.
async fn test_db() -> Option<DatabaseConnection> {
    let Ok(url) = std::env::var("KOJI_DB_URL") else {
        eprintln!("skip: KOJI_DB_URL unset");
        return None;
    };
    match Database::connect(&url).await {
        Ok(db) => Some(db),
        Err(e) => {
            eprintln!("skip: could not connect to KOJI_DB_URL: {e}");
            None
        }
    }
}

/// A short, unique `kind` for one test run — `t-<ulid>` (≤ 64 chars). Used both
/// as the handler kind and as the cleanup `DELETE` predicate so a test only ever
/// touches its own rows.
fn unique_kind(tag: &str) -> String {
    format!("test-{tag}-{}", JobId::new().as_string())
}

/// Delete every `job` row with the given `kind` (best-effort cleanup).
async fn cleanup(db: &DatabaseConnection, kind: &str) {
    let _ = db
        .execute(Statement::from_sql_and_values(
            DbBackend::MySql,
            "DELETE FROM `job` WHERE `kind` = ?",
            [Value::from(kind.to_owned())],
        ))
        .await;
}

#[tokio::test]
async fn enqueue_then_get_is_queued() {
    let Some(db) = test_db().await else { return };
    let _serial = serial_guard().await; // serialize: see SERIAL doc
    let kind = unique_kind("enq");
    let q = JobQueue::new(db.clone(), "test-worker");

    let id = q
        .enqueue_or_attach(&kind, None, &serde_json::json!({ "n": 1 }), 0)
        .await
        .expect("enqueue should succeed");

    // Capture, then clean up, then assert — so a failing assertion can't strand
    // the job row (`#[tokio::test]` has no teardown hook).
    let rec = q.get(id).await;
    cleanup(&db, &kind).await;

    let rec = rec.expect("get should find the job");
    assert_eq!(rec.id, id, "get returns the same id");
    assert_eq!(rec.kind, kind, "kind round-trips");
    assert_eq!(rec.status, JobStatus::Queued, "fresh job is Queued");
    assert_eq!(rec.progress, 0.0, "fresh job has 0 progress");
}

#[tokio::test]
async fn enqueue_or_attach_coalesces_on_dedup_key() {
    let Some(db) = test_db().await else { return };
    let _serial = serial_guard().await; // serialize: see SERIAL doc
    let kind = unique_kind("dedup");
    let q = JobQueue::new(db.clone(), "test-worker");

    // A dedup_key unique to this run (content-addressed off the unique kind).
    let key = dedup_key(&kind, &serde_json::json!({ "area": "north" }));

    let id1 = q
        .enqueue_or_attach(&kind, Some(&key), &serde_json::json!({ "x": 1 }), 0)
        .await
        .expect("first enqueue");
    let id2 = q
        .enqueue_or_attach(&kind, Some(&key), &serde_json::json!({ "x": 2 }), 0)
        .await
        .expect("second enqueue with same key");

    // Observe the row count for this kind, then clean up, then assert.
    let listed = q.list(1, 50, Some(JobStatus::Queued)).await;
    cleanup(&db, &kind).await;

    assert_eq!(
        id1, id2,
        "same dedup_key while in-flight must coalesce to the same JobId"
    );

    // And exactly one row was inserted for this kind.
    let (rows, total) = listed.expect("list");
    let mine = rows.iter().filter(|r| r.kind == kind).count();
    assert_eq!(mine, 1, "coalesced enqueue inserts exactly one row");
    assert!(total >= 1, "total counts at least our job");
}

#[tokio::test]
async fn claim_marks_running_and_returns_payload() {
    let Some(db) = test_db().await else { return };
    let _serial = serial_guard().await; // serialize: see SERIAL doc
    let kind = unique_kind("claim");
    let q = JobQueue::new(db.clone(), "test-worker-claim");

    // Enqueue at MAX priority so our row sorts to the front of the global queue
    // (`claim` orders by `priority DESC, id ASC`). No other test uses max
    // priority, so nothing outranks ours — the first claim that reaches a
    // *visible, unlocked* row returns ours. This removes the flake where a
    // concurrent test's priority-0 job (enqueued earlier, lower id) would be
    // handed out first and starve this loop.
    let payload = serde_json::json!({ "area": "central", "k": 42 });
    let id = q
        .enqueue_or_attach(&kind, None, &payload, i16::MAX)
        .await
        .expect("enqueue");

    // Claim until we get ours. A `Some(stray)` (another test's job, momentarily
    // higher-priority is impossible but a same-id race could still surface one)
    // is parked Running under our worker and cleaned by its owner — keep going.
    // A transient `None` means every queued row (incl. ours) is SKIP-LOCKED by a
    // concurrent worker this instant — yield and retry rather than give up.
    let mut claimed = None;
    for _ in 0..200 {
        match q.claim().await.expect("claim should not error") {
            Some(c) if c.kind == kind => {
                claimed = Some(c);
                break;
            }
            Some(_) => continue,
            None => tokio::task::yield_now().await,
        }
    }

    // Capture the post-claim state, then clean up, then assert (panic-safe).
    let rec = q.get(id).await;
    cleanup(&db, &kind).await;

    let claimed = claimed.expect("our queued job should be claimable");
    assert_eq!(claimed.public_id, id, "claim returns our public id");
    assert_eq!(claimed.kind, kind, "claim returns our kind");
    assert_eq!(claimed.payload, payload, "claim returns the exact payload");
    assert_eq!(claimed.attempts, 1, "first claim sets attempts = 1");

    let rec = rec.expect("get after claim");
    assert_eq!(rec.status, JobStatus::Running, "claimed job is Running");
}

#[tokio::test]
async fn cancel_queued_job_becomes_canceled() {
    let Some(db) = test_db().await else { return };
    let _serial = serial_guard().await; // serialize: see SERIAL doc
    let kind = unique_kind("cancel");
    let q = JobQueue::new(db.clone(), "test-worker");

    let id = q
        .enqueue_or_attach(&kind, None, &serde_json::json!({}), 0)
        .await
        .expect("enqueue");

    q.cancel(id).await.expect("cancel a queued job");

    let rec = q.get(id).await;
    cleanup(&db, &kind).await;

    let rec = rec.expect("get after cancel");
    assert_eq!(
        rec.status,
        JobStatus::Canceled,
        "a still-queued job goes straight to Canceled"
    );
}

#[tokio::test]
async fn list_contains_enqueued_job_with_sane_total() {
    let Some(db) = test_db().await else { return };
    let _serial = serial_guard().await; // serialize: see SERIAL doc
    let kind = unique_kind("list");
    let q = JobQueue::new(db.clone(), "test-worker");

    let id = q
        .enqueue_or_attach(&kind, None, &serde_json::json!({ "z": true }), 0)
        .await
        .expect("enqueue");

    // Unfiltered list, newest-first — our just-enqueued job is on page 1.
    let listed = q.list(1, 50, None).await;
    cleanup(&db, &kind).await;

    let (rows, total) = listed.expect("list");
    assert!(
        rows.iter().any(|r| r.id == id && r.kind == kind),
        "list should contain the freshly enqueued job"
    );
    assert!(
        total >= 1,
        "total should count at least our job, got {total}"
    );
}

#[tokio::test]
async fn await_result_returns_succeeded_after_terminal_update() {
    let Some(db) = test_db().await else { return };
    let _serial = serial_guard().await; // serialize: see SERIAL doc
    let kind = unique_kind("await");
    let q = JobQueue::new(db.clone(), "test-worker");

    let id = q
        .enqueue_or_attach(&kind, None, &serde_json::json!({}), 0)
        .await
        .expect("enqueue");

    // Mark the job succeeded out-of-band via the SAME connection, exactly as a
    // worker in another process would (raw terminal UPDATE).
    let result = serde_json::json!({ "ok": true, "count": 7 });
    db.execute(Statement::from_sql_and_values(
        DbBackend::MySql,
        "UPDATE `job` SET `status` = 'succeeded', `result` = ?, `finished_at` = NOW() \
         WHERE `public_id` = ?",
        [Value::from(result.clone()), Value::from(id.as_string())],
    ))
    .await
    .expect("terminal UPDATE should succeed");

    // await_result short-circuits on the already-terminal row.
    let outcome = q.await_result(id, Duration::from_secs(2)).await;
    cleanup(&db, &kind).await;

    match outcome.expect("await_result on a terminal job") {
        JobOutcome::Succeeded(v) => assert_eq!(v, result, "Succeeded carries the result JSON"),
        other => panic!("expected Succeeded, got {other:?}"),
    }
}
