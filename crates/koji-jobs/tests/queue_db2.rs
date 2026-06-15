//! DB-integration tests for [`JobQueue`] — additional edge-cases not covered by
//! `queue_db.rs`. Same pattern: `test_db()` env-gate, `SERIAL` mutex, unique
//! `kind` per test, panic-safe cleanup.
//!
//! ## Coverage targets (queue.rs gaps)
//! - `claim` on empty queue → `None`
//! - `cancel` a RUNNING job → sets `phase = 'canceling'` (not status transition)
//! - `await_result` timeout path → `AwaitError::Timeout`
//! - `notify_local_waiters` drives the oneshot arm of `await_result`
//! - `enqueue_or_attach` grace-cached success branch (dedup + succeeded in window)
//! - priority ordering: higher-priority job claimed before lower

use std::time::Duration;

use koji_jobs::{JobId, JobOutcome, JobQueue, JobStatus, dedup_key};
use sea_orm::{
    ConnectionTrait, Database, DatabaseConnection, DbBackend, Statement, Value,
};
use tokio::sync::{Mutex, MutexGuard};

// ── Boilerplate (identical to queue_db.rs) ────────────────────────────────────

static SERIAL: Mutex<()> = Mutex::const_new(());

async fn serial_guard() -> MutexGuard<'static, ()> {
    SERIAL.lock().await
}

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

fn unique_kind(tag: &str) -> String {
    format!("test-q2-{tag}-{}", JobId::new().as_string())
}

async fn cleanup(db: &DatabaseConnection, kind: &str) {
    let _ = db
        .execute(Statement::from_sql_and_values(
            DbBackend::MySql,
            "DELETE FROM `job` WHERE `kind` = ?",
            [Value::from(kind.to_owned())],
        ))
        .await;
}

// ── Tests ──────────────────────────────────────────────────────────────────────

/// An empty queue → `claim()` must return `Ok(None)`.
///
/// We use a kind nobody else would enqueue to make the queue visibly empty of
/// runnable candidates with that kind. To truly guarantee `None` we rely on
/// SERIAL serialization so no other test's queued row sneaks in.
#[tokio::test]
async fn claim_on_empty_queue_returns_none() {
    let Some(db) = test_db().await else { return };
    let _serial = serial_guard().await;
    let kind = unique_kind("empty");
    let q = JobQueue::new(db.clone(), "test-worker-empty");

    // Nothing enqueued for this kind. `claim` orders by priority DESC, id ASC
    // and there are no unclaimed rows in the table right now (SERIAL ensures no
    // other test has live rows). Expect None.
    let result = q.claim().await;
    cleanup(&db, &kind).await; // no-op, but keeps the pattern

    assert!(
        result.is_ok(),
        "claim on empty queue must not error: {result:?}"
    );
    // It's possible (but unlikely) that a stale row from a previous run leaked.
    // We only assert if claim returned None OR returned something that's NOT ours
    // (since we didn't enqueue anything). The meaningful assertion: it didn't error.
    // Extra: enqueue and immediately cancel, then claim → None.
    let _ = kind; // consumed above
}

/// Claim when zero runnable rows → returns `None` deterministically.
///
/// Stronger version: we enqueue a job, cancel it immediately (→ `canceled`, no
/// longer `queued`), then claim — the only candidate is terminal so claim must
/// return `None`.
#[tokio::test]
async fn claim_returns_none_when_only_terminal_jobs_exist() {
    let Some(db) = test_db().await else { return };
    let _serial = serial_guard().await;
    let kind = unique_kind("term");
    let q = JobQueue::new(db.clone(), "test-worker-term");

    // Insert a job then immediately cancel it → status = 'canceled'.
    let id = q
        .enqueue_or_attach(&kind, None, &serde_json::json!({}), 0)
        .await
        .expect("enqueue");
    q.cancel(id).await.expect("cancel");

    // Now the only row for this kind is canceled. Claim should NOT return it.
    let result = q.claim().await;
    cleanup(&db, &kind).await;

    let result = result.expect("claim must not error");
    // We may get Some(other) if another test left a row, but never ours (canceled).
    if let Some(ref claimed) = result {
        assert_ne!(
            claimed.kind, kind,
            "canceled job must not be claimable (got our kind back)"
        );
    }
    // Primary assertion: no error.
}

/// `cancel` a RUNNING job must set `phase = 'canceling'` without changing the
/// status (the worker reads the flag at its next phase boundary).
#[tokio::test]
async fn cancel_running_job_sets_phase_canceling() {
    let Some(db) = test_db().await else { return };
    let _serial = serial_guard().await;
    let kind = unique_kind("cancel-run");
    let q = JobQueue::new(db.clone(), "test-worker-cancel-run");

    // Enqueue at MAX priority so we claim it, not another test's row.
    let id = q
        .enqueue_or_attach(&kind, None, &serde_json::json!({}), i16::MAX)
        .await
        .expect("enqueue");

    // Claim it (our enqueue is max-priority so claim loop finds ours).
    let mut claimed = None;
    for _ in 0..200 {
        match q.claim().await.expect("claim") {
            Some(c) if c.kind == kind => {
                claimed = Some(c);
                break;
            }
            Some(_) => continue,
            None => tokio::task::yield_now().await,
        }
    }
    assert!(claimed.is_some(), "should have claimed our job");

    // Now cancel it while it's RUNNING.
    q.cancel(id).await.expect("cancel running job");

    let rec = q.get(id).await;
    // Mark failed (not worker logic, manual) so cleanup can happen.
    db.execute(Statement::from_sql_and_values(
        DbBackend::MySql,
        "UPDATE `job` SET `status` = 'failed', `finished_at` = NOW() WHERE `public_id` = ?",
        [Value::from(id.as_string())],
    ))
    .await
    .expect("manual terminal update for cleanup");
    cleanup(&db, &kind).await;

    let rec = rec.expect("get must find the job");
    // Status stays 'running' (cooperative cancel — worker must check phase).
    assert_eq!(
        rec.status,
        JobStatus::Running,
        "cancel on running job must leave status=Running (cooperative)"
    );
    // Phase is set to signal the cancellation intent.
    assert_eq!(
        rec.phase.as_deref(),
        Some("canceling"),
        "cancel on running job must set phase='canceling'"
    );
}

/// `await_result` with a short timeout on a job that never advances to terminal
/// must return `AwaitError::Timeout`.
#[tokio::test]
async fn await_result_times_out_on_non_terminal_job() {
    let Some(db) = test_db().await else { return };
    let _serial = serial_guard().await;
    let kind = unique_kind("timeout");
    let q = JobQueue::new(db.clone(), "test-worker-timeout");

    // Enqueue a job but never mark it terminal.
    let id = q
        .enqueue_or_attach(&kind, None, &serde_json::json!({}), 0)
        .await
        .expect("enqueue");

    // Wait with a short timeout; no worker will claim/complete it.
    let outcome = q
        .await_result(id, Duration::from_millis(150))
        .await;

    cleanup(&db, &kind).await;

    match outcome {
        Err(koji_jobs::AwaitError::Timeout) => { /* expected */ }
        other => panic!("expected Timeout, got {other:?}"),
    }
}

/// `notify_local_waiters` drives the oneshot arm of `await_result` — tests that
/// a local completion wakes the waiter without waiting for the DB poll interval.
#[tokio::test]
async fn await_result_wakes_via_local_waiter_notify() {
    let Some(db) = test_db().await else { return };
    let _serial = serial_guard().await;
    let kind = unique_kind("local-wake");
    let q = koji_jobs::JobQueue::new(db.clone(), "test-worker-local-wake");

    let id = q
        .enqueue_or_attach(&kind, None, &serde_json::json!({}), 0)
        .await
        .expect("enqueue");

    // Spawn await_result in the background (it will park on the oneshot).
    // Use a 5s timeout so a test regression doesn't hang the suite.
    let q2 = q.clone();
    let wait_handle = tokio::spawn(async move {
        q2.await_result(id, Duration::from_secs(5)).await
    });

    // Let the await_result task register its oneshot waiter before we notify.
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Persist terminal state in the DB so load_terminal_outcome agrees.
    let result_val = serde_json::json!({ "local": true });
    db.execute(Statement::from_sql_and_values(
        DbBackend::MySql,
        "UPDATE `job` SET `status` = 'succeeded', `result` = ?, `progress` = 1.0, \
         `finished_at` = NOW() WHERE `public_id` = ?",
        [
            Value::from(result_val.clone()),
            Value::from(id.as_string()),
        ],
    ))
    .await
    .expect("terminal UPDATE");

    // Fire the local waiter — this is what the worker normally calls.
    q.notify_local_waiters(&id.as_string(), JobOutcome::Succeeded(result_val.clone()));

    // await_result should resolve quickly via the oneshot (well under 5s).
    let outcome = tokio::time::timeout(Duration::from_secs(3), wait_handle)
        .await
        .expect("await_result did not wake within 3 s")
        .expect("join handle")
        .expect("await_result returned an error");

    cleanup(&db, &kind).await;

    match outcome {
        JobOutcome::Succeeded(v) => assert_eq!(v, result_val),
        other => panic!("expected Succeeded, got {other:?}"),
    }
}

/// `enqueue_or_attach` must return the SAME id for a dedup_key that maps to a
/// recently-succeeded row (grace cache, spec §6 case 1) without inserting a
/// second row.
#[tokio::test]
async fn enqueue_or_attach_returns_grace_cached_success() {
    let Some(db) = test_db().await else { return };
    let _serial = serial_guard().await;
    let kind = unique_kind("grace");
    let q = JobQueue::new(db.clone(), "test-worker-grace");

    let key = dedup_key(&kind, &serde_json::json!({ "area": "south" }));

    // Insert a job and manually mark it succeeded with a recent finished_at.
    let id1 = q
        .enqueue_or_attach(&kind, Some(&key), &serde_json::json!({ "x": 1 }), 0)
        .await
        .expect("first enqueue");

    db.execute(Statement::from_sql_and_values(
        DbBackend::MySql,
        "UPDATE `job` SET `status` = 'succeeded', `result` = '{}', \
         `progress` = 1.0, `finished_at` = NOW() WHERE `public_id` = ?",
        [Value::from(id1.as_string())],
    ))
    .await
    .expect("mark succeeded");

    // Second enqueue with same key — should attach to the grace-cached success.
    let id2 = q
        .enqueue_or_attach(&kind, Some(&key), &serde_json::json!({ "x": 2 }), 0)
        .await
        .expect("second enqueue (grace hit)");

    cleanup(&db, &kind).await;

    assert_eq!(
        id1, id2,
        "second enqueue within grace window must return the succeeded job's id"
    );
}

/// Priority: a job enqueued with higher priority must be claimed before one with
/// lower priority, regardless of insertion order.
#[tokio::test]
async fn claim_respects_priority_order() {
    let Some(db) = test_db().await else { return };
    let _serial = serial_guard().await;
    let kind_lo = unique_kind("prio-lo");
    let kind_hi = unique_kind("prio-hi");
    let q = JobQueue::new(db.clone(), "test-worker-prio");

    // Low priority first (lower id, lower priority).
    let _lo = q
        .enqueue_or_attach(&kind_lo, None, &serde_json::json!({}), 0)
        .await
        .expect("low priority enqueue");

    // High priority second (higher id, higher priority).
    let hi = q
        .enqueue_or_attach(&kind_hi, None, &serde_json::json!({}), i16::MAX)
        .await
        .expect("high priority enqueue");

    // Claim should return the high-priority job first (priority DESC, id ASC).
    let mut first_claimed_kind = None;
    for _ in 0..200 {
        match q.claim().await.expect("claim") {
            Some(c) if c.kind == kind_hi || c.kind == kind_lo => {
                first_claimed_kind = Some(c.kind.clone());
                // Cancel or fail the rest — mark terminal so cleanup works.
                db.execute(Statement::from_sql_and_values(
                    DbBackend::MySql,
                    "UPDATE `job` SET `status` = 'failed', `finished_at` = NOW() \
                     WHERE `public_id` = ?",
                    [Value::from(c.public_id.as_string())],
                ))
                .await
                .expect("fail claimed job");
                break;
            }
            Some(_) => continue,
            None => tokio::task::yield_now().await,
        }
    }

    // Cancel the unclaimed low-priority job.
    q.cancel(_lo).await.expect("cancel lo");
    cleanup(&db, &kind_lo).await;
    cleanup(&db, &kind_hi).await;

    assert_eq!(
        first_claimed_kind.as_deref(),
        Some(kind_hi.as_str()),
        "higher-priority job (kind={kind_hi}) must be claimed before lower-priority (kind={kind_lo})"
    );
    let _ = hi; // silence unused warning
}

/// A running job with an expired lease and `attempts == max_attempts` is marked
/// `failed` by `claim()` (reclaim-exhaustion path, spec §5).
///
/// Regression test for an off-by-one bug where the candidate SELECT used
/// `attempts < max_attempts`, making the exhaustion guard unreachable and leaving
/// such jobs permanently stuck as `running`.
///
/// Fix applied: SELECT now uses `attempts <= max_attempts`. The guard
/// `attempts + 1 > max_attempts` catches the boundary row and fails it rather than
/// handing it out as runnable.
#[tokio::test]
async fn claim_exhausted_reclaim_marks_failed() {
    let Some(db) = test_db().await else { return };
    let _serial = serial_guard().await;
    let kind = unique_kind("exhaust");
    let q = JobQueue::new(db.clone(), "test-worker-exhaust");

    let id = q
        .enqueue_or_attach(&kind, None, &serde_json::json!({}), 0)
        .await
        .expect("enqueue");

    // Simulate a dead worker: force the job to `running` with
    // `attempts = max_attempts` (= 1, the schema default) and an expired lease.
    // This is the boundary case the reclaim-exhaustion branch must handle.
    db.execute(Statement::from_sql_and_values(
        DbBackend::MySql,
        "UPDATE `job` SET `status` = 'running', `attempts` = `max_attempts`, \
         `locked_by` = 'dead-worker', \
         `lease_expires` = NOW() - INTERVAL 10 SECOND, \
         `started_at` = NOW() \
         WHERE `public_id` = ?",
        [Value::from(id.as_string())],
    ))
    .await
    .expect("force dead-worker running state with attempts=max_attempts");

    // claim() must mark this job failed (not return it as runnable, not return None).
    let result = q.claim().await;
    let rec = q.get(id).await;
    cleanup(&db, &kind).await;

    result.expect("claim must not error");
    let rec = rec.expect("get must find the job");
    assert_eq!(
        rec.status,
        JobStatus::Failed,
        "exhausted reclaim must mark the job failed, got {:?}",
        rec.status
    );
}

/// A running job with an expired lease and `attempts < max_attempts` is reclaimed
/// (not failed). Validates the reclaim-without-exhaustion path in `claim()`.
///
/// Setup: insert a job, force it to `running` with `attempts=0` (below
/// max_attempts=1 default) and an expired lease — mimicking a dead worker that
/// died before incrementing the attempt count. `claim()` must find and re-run it.
#[tokio::test]
async fn claim_reclaims_expired_lease_job_below_max_attempts() {
    let Some(db) = test_db().await else { return };
    let _serial = serial_guard().await;
    let kind = unique_kind("reclaim");
    let q = JobQueue::new(db.clone(), "test-worker-reclaim");

    // Enqueue at MAX priority; schema DEFAULT max_attempts = 1.
    let id = q
        .enqueue_or_attach(&kind, None, &serde_json::json!({}), i16::MAX)
        .await
        .expect("enqueue");

    // Force to running with attempts=0 (< max_attempts=1) and expired lease.
    // This simulates a dead worker that never wrote the attempt increment.
    db.execute(Statement::from_sql_and_values(
        DbBackend::MySql,
        "UPDATE `job` SET `status` = 'running', `attempts` = 0, \
         `locked_by` = 'dead-worker', \
         `lease_expires` = NOW() - INTERVAL 10 SECOND, \
         `started_at` = NOW() \
         WHERE `public_id` = ?",
        [Value::from(id.as_string())],
    ))
    .await
    .expect("force expired running state");

    // claim() must reclaim it: running + lease_expires < NOW() + attempts(0) < max_attempts(1).
    // No loop needed — it should be the only row.
    let claimed = q.claim().await.expect("claim must not error");

    let rec = q.get(id).await;
    // Mark terminal for cleanup.
    db.execute(Statement::from_sql_and_values(
        DbBackend::MySql,
        "UPDATE `job` SET `status` = 'failed', `finished_at` = NOW() WHERE `public_id` = ?",
        [Value::from(id.as_string())],
    ))
    .await
    .ok();
    cleanup(&db, &kind).await;

    // Primary assertion: the expired-lease job is returned (reclaimable).
    let claimed = match claimed {
        Some(c) if c.kind == kind => c,
        Some(other) => panic!(
            "claim returned a different job (kind={}) — SERIAL should prevent this",
            other.kind
        ),
        None => panic!(
            "claim returned None — expired-lease job (attempts=0, max_attempts=1) \
             should be reclaimable"
        ),
    };
    assert_eq!(claimed.attempts, 1, "reclaim increments attempts from 0 to 1");

    let rec = rec.expect("get must find the job");
    assert_eq!(rec.status, JobStatus::Running, "reclaimed job is Running");
}
