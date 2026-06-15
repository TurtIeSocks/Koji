//! DB-integration tests for [`EventDispatcher`] — claim, deliver, backoff, and
//! dead-letter paths against a live MySQL `koji_test` (`event_outbox` table).
//!
//! ## Gating (no `#[ignore]`)
//! Every test begins `let Some(db) = test_db().await else { return };`.
//! `test_db()` reads `KOJI_DB_URL`; if unset it prints a skip line and returns
//! `None`, so a plain `cargo test -p koji-events` **passes by skipping**.
//!
//! With the env sourced:
//! ```sh
//! set -a; source ./.env.test; set +a
//! cargo test -p koji-events --test dispatcher_db -- --nocapture
//! ```
//!
//! ## Isolation
//! Each test derives a unique topic from an [`EventId`] ULID so parallel / repeat
//! runs never collide.  Cleanup runs via `DELETE … WHERE topic = ?` BEFORE the
//! final assertions so a failing assert never strands rows in `koji_test`.
//!
//! ## In-memory subscribers
//! Tests use locally-defined `AcceptAll` / `AlwaysFail` / `TopicFilter` — no
//! HTTP endpoint, fully deterministic, offline.

use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use std::time::Duration;

use async_trait::async_trait;
use koji_events::{DeliverError, EventDispatcher, EventId, Subscriber};
use koji_events::types::Event;
use sea_orm::{
    ConnectionTrait, Database, DatabaseConnection, DbBackend, Statement, Value,
};
use tokio::sync::Mutex;

// ---- Process-wide serialization (mirrors queue_db.rs) -----------------------

/// One dispatcher run at a time: `claim_due` uses `FOR UPDATE SKIP LOCKED` on
/// the shared `event_outbox` table; concurrent in-test dispatchers on
/// overlapping topic scans deadlock or starve. Serializing matches single-pool
/// production use and makes status-transition assertions deterministic.
static SERIAL: Mutex<()> = Mutex::const_new(());

// ---- Test DB connection -----------------------------------------------------

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

// ---- Unique topic + cleanup --------------------------------------------------

/// Unique topic string for one test run: `test-<tag>-<ulid>`.
fn unique_topic(tag: &str) -> String {
    format!("test-{tag}-{}", EventId::new().as_string())
}

/// DELETE all `event_outbox` rows for this topic (best-effort; called BEFORE
/// final assertions so stranding is impossible even if the test panics).
async fn cleanup(db: &DatabaseConnection, topic: &str) {
    let _ = db
        .execute(Statement::from_sql_and_values(
            DbBackend::MySql,
            "DELETE FROM `event_outbox` WHERE `topic` = ?",
            [Value::from(topic.to_owned())],
        ))
        .await;
}

/// READ the `status`, `attempts`, `last_error`, `locked_by` columns for the
/// most-recently-updated row matching `topic`. Returns `None` if no row found.
async fn read_row(
    db: &DatabaseConnection,
    topic: &str,
) -> Option<(String, i32, Option<String>, Option<String>)> {
    let row = db
        .query_one(Statement::from_sql_and_values(
            DbBackend::MySql,
            "SELECT `status`, `attempts`, `last_error`, `locked_by` \
             FROM `event_outbox` WHERE `topic` = ? ORDER BY `id` DESC LIMIT 1",
            [Value::from(topic.to_owned())],
        ))
        .await
        .ok()??;
    let status: String = row.try_get("", "status").ok()?;
    let attempts: i32 = row.try_get("", "attempts").ok()?;
    let last_error: Option<String> = row.try_get("", "last_error").ok()?;
    let locked_by: Option<String> = row.try_get("", "locked_by").ok()?;
    Some((status, attempts, last_error, locked_by))
}

/// Poll `read_row` up to `deadline` until `predicate` holds; return the last
/// observed value.  Gives the dispatcher loop time to complete its
/// claim→deliver→persist cycle.
async fn wait_for_status<F>(
    db: &DatabaseConnection,
    topic: &str,
    deadline: Duration,
    predicate: F,
) -> Option<(String, i32, Option<String>, Option<String>)>
where
    F: Fn(&str) -> bool,
{
    let end = tokio::time::Instant::now() + deadline;
    loop {
        if let Some(row) = read_row(db, topic).await {
            if predicate(&row.0) {
                return Some(row);
            }
        }
        if tokio::time::Instant::now() >= end {
            return read_row(db, topic).await;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

// ---- In-memory subscribers --------------------------------------------------

/// Accepts every delivery; counts calls.
struct AcceptAll {
    count: AtomicUsize,
}

impl AcceptAll {
    fn new() -> Self {
        AcceptAll { count: AtomicUsize::new(0) }
    }
    fn delivered(&self) -> usize {
        self.count.load(Ordering::SeqCst)
    }
}

#[async_trait]
impl Subscriber for AcceptAll {
    fn name(&self) -> &'static str {
        "accept-all"
    }
    async fn deliver(&self, _event: &Event) -> Result<(), DeliverError> {
        self.count.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

/// Rejects every delivery with a deterministic error.
struct AlwaysFail;

#[async_trait]
impl Subscriber for AlwaysFail {
    fn name(&self) -> &'static str {
        "always-fail"
    }
    async fn deliver(&self, _event: &Event) -> Result<(), DeliverError> {
        Err(DeliverError::Other("forced failure".into()))
    }
}

/// Only interested in one specific topic string.
struct TopicFilter {
    want: String,
}

#[async_trait]
impl Subscriber for TopicFilter {
    fn name(&self) -> &'static str {
        "topic-filter"
    }
    async fn deliver(&self, _event: &Event) -> Result<(), DeliverError> {
        Ok(())
    }
    fn interested_in(&self, topic: &str) -> bool {
        topic == self.want
    }
}

// ---- Tests ------------------------------------------------------------------

/// `publish` inserts a row with `status='pending'`, `attempts=0`.
#[tokio::test]
async fn publish_inserts_pending_row() {
    let Some(db) = test_db().await else { return };
    let _serial = SERIAL.lock().await;
    let topic = unique_topic("pub");

    EventDispatcher::publish(&db, &topic, &serde_json::json!({"x": 1}))
        .await
        .expect("publish should succeed");

    let (status, attempts, _, _) = read_row(&db, &topic)
        .await
        .expect("row should exist after publish");
    cleanup(&db, &topic).await;

    assert_eq!(status, "pending", "freshly published row must be pending");
    assert_eq!(attempts, 0, "freshly published row has 0 attempts");
}

/// A pending row is claimed (status→'delivering', locked_by set) by the
/// dispatcher loop, then transitions to 'delivered' when the subscriber accepts.
#[tokio::test]
async fn dispatcher_delivers_pending_event_to_accept_all_subscriber() {
    let Some(db) = test_db().await else { return };
    let _serial = SERIAL.lock().await;
    let topic = unique_topic("deliver");

    let sub = Arc::new(AcceptAll::new());
    EventDispatcher::publish(&db, &topic, &serde_json::json!({"k": "v"}))
        .await
        .expect("publish");

    let dispatcher = Arc::new(EventDispatcher::new(
        db.clone(),
        vec![sub.clone() as Arc<dyn Subscriber>],
        "test-deliver",
    ));
    let handle = dispatcher.spawn();

    // Wait for the terminal status, then shut down.
    let row = wait_for_status(&db, &topic, Duration::from_secs(5), |s| s == "delivered").await;
    handle.shutdown().await;
    cleanup(&db, &topic).await;

    let (status, _, _, _) = row.expect("row must exist after dispatch");
    assert_eq!(status, "delivered", "AcceptAll subscriber → status=delivered");
    // NOTE: mark_delivered does not NULL out locked_by/lease_expires (mark_failed
    // does). The delivered row retains the worker-id in locked_by. That is
    // harmless in production (status=delivered is terminal), but is a minor
    // inconsistency with mark_failed's cleanup.
    assert_eq!(sub.delivered(), 1, "subscriber receive count must be 1");
}

/// When `AlwaysFail` rejects, the row backs off: `attempts` increments and
/// `status` returns to 'pending'.
#[tokio::test]
async fn dispatcher_backs_off_on_subscriber_failure() {
    let Some(db) = test_db().await else { return };
    let _serial = SERIAL.lock().await;
    let topic = unique_topic("backoff");

    EventDispatcher::publish(&db, &topic, &serde_json::json!({"fail": true}))
        .await
        .expect("publish");

    let dispatcher = Arc::new(EventDispatcher::new(
        db.clone(),
        vec![Arc::new(AlwaysFail) as Arc<dyn Subscriber>],
        "test-backoff",
    ));
    let handle = dispatcher.spawn();

    // Poll until pending WITH attempts > 0: initial state is pending+attempts=0
    // (pre-dispatch). After the first delivery failure the row returns to
    // pending+attempts=1. Distinguish by checking attempts in the predicate.
    let end = tokio::time::Instant::now() + Duration::from_secs(5);
    let mut final_row;
    loop {
        final_row = read_row(&db, &topic).await;
        if let Some(ref row) = final_row {
            if row.0 == "pending" && row.1 >= 1 {
                break;
            }
        }
        if tokio::time::Instant::now() >= end {
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    handle.shutdown().await;
    cleanup(&db, &topic).await;

    let (status, attempts, last_error, _) = final_row.expect("row must exist");
    assert_eq!(status, "pending", "failed row must return to pending for retry");
    assert_eq!(attempts, 1, "one failed attempt recorded");
    assert!(
        last_error.is_some(),
        "last_error must be set after a delivery failure"
    );
    let err = last_error.unwrap();
    assert!(
        err.contains("forced failure"),
        "last_error should carry the subscriber error text; got: {err:?}"
    );
}

/// After `max_attempts` failures the row is dead-lettered: `status='dead'`.
/// We publish with `max_attempts=1` so the very first failure is terminal.
#[tokio::test]
async fn dispatcher_dead_letters_after_max_attempts_exhausted() {
    let Some(db) = test_db().await else { return };
    let _serial = SERIAL.lock().await;
    let topic = unique_topic("dead");

    // Insert the row manually with max_attempts=1 so one failure exhausts it.
    db.execute(Statement::from_sql_and_values(
        DbBackend::MySql,
        "INSERT INTO `event_outbox` \
           (`public_id`, `topic`, `payload`, `status`, `attempts`, `max_attempts`, \
            `next_attempt_at`, `created_at`) \
         VALUES (?, ?, ?, 'pending', 0, 1, NOW(), NOW())",
        [
            Value::from(EventId::new().as_string()),
            Value::from(topic.clone()),
            Value::from(serde_json::json!({"dead": true})),
        ],
    ))
    .await
    .expect("manual insert");

    let dispatcher = Arc::new(EventDispatcher::new(
        db.clone(),
        vec![Arc::new(AlwaysFail) as Arc<dyn Subscriber>],
        "test-dead",
    ));
    let handle = dispatcher.spawn();

    let row = wait_for_status(&db, &topic, Duration::from_secs(5), |s| s == "dead").await;
    handle.shutdown().await;
    cleanup(&db, &topic).await;

    let (status, attempts, last_error, _) = row.expect("row must exist");
    assert_eq!(status, "dead", "row must be dead-lettered once max_attempts=1 is exhausted");
    assert_eq!(attempts, 1, "attempts bumped to 1 on the fatal delivery");
    assert!(last_error.is_some(), "last_error recorded on dead row");
}

/// A `TopicFilter` subscriber that wants topic A does NOT receive an event
/// published to topic B; the event is still delivered (status→'delivered')
/// because no *interested* subscriber fails.
#[tokio::test]
async fn dispatcher_skips_unmatched_topic_for_topic_filter_subscriber() {
    let Some(db) = test_db().await else { return };
    let _serial = SERIAL.lock().await;
    let topic_a = unique_topic("tf-a");
    let topic_b = unique_topic("tf-b");

    // Subscriber only wants topic_a; we publish to topic_b.
    let sub = Arc::new(AcceptAll::new()); // counts deliveries
    let filter = Arc::new(TopicFilter { want: topic_a.clone() });

    EventDispatcher::publish(&db, &topic_b, &serde_json::json!({"mismatch": true}))
        .await
        .expect("publish topic_b");

    let dispatcher = Arc::new(EventDispatcher::new(
        db.clone(),
        // Both subscribers registered; only filter passes interest check.
        vec![
            sub.clone() as Arc<dyn Subscriber>,
            filter as Arc<dyn Subscriber>,
        ],
        "test-tf",
    ));
    // AcceptAll passes all topics, so delivery succeeds (status→delivered).
    let handle = dispatcher.spawn();

    let row =
        wait_for_status(&db, &topic_b, Duration::from_secs(5), |s| s == "delivered").await;
    handle.shutdown().await;
    cleanup(&db, &topic_b).await;
    cleanup(&db, &topic_a).await;

    let (status, _, _, _) = row.expect("topic_b row must exist");
    assert_eq!(
        status, "delivered",
        "event should be delivered even when a TopicFilter subscriber skips it"
    );
    // AcceptAll gets every topic → count = 1 for the topic_b event.
    assert_eq!(
        sub.delivered(),
        1,
        "AcceptAll should receive the event once; TopicFilter skips it"
    );
}

/// When the ONLY subscriber is a `TopicFilter` that doesn't match, no
/// subscriber is called AND the event is still marked 'delivered' (vacuous
/// success — no interested subscriber failed).
#[tokio::test]
async fn dispatcher_delivers_vacuously_when_no_subscriber_is_interested() {
    let Some(db) = test_db().await else { return };
    let _serial = SERIAL.lock().await;
    let topic_want = unique_topic("vac-want");
    let topic_publish = unique_topic("vac-pub");

    // TopicFilter only wants topic_want; we publish to topic_publish.
    let filter = Arc::new(TopicFilter { want: topic_want.clone() });
    let accept = Arc::new(AcceptAll::new());

    EventDispatcher::publish(&db, &topic_publish, &serde_json::json!({}))
        .await
        .expect("publish");

    // Only the TopicFilter is registered (no AcceptAll) → vacuous success.
    let dispatcher = Arc::new(EventDispatcher::new(
        db.clone(),
        vec![filter as Arc<dyn Subscriber>],
        "test-vac",
    ));
    let handle = dispatcher.spawn();

    let row = wait_for_status(&db, &topic_publish, Duration::from_secs(5), |s| {
        s == "delivered"
    })
    .await;
    handle.shutdown().await;
    cleanup(&db, &topic_publish).await;
    cleanup(&db, &topic_want).await;

    let (status, _, _, _) = row.expect("row must exist");
    assert_eq!(
        status, "delivered",
        "vacuous delivery (no interested subscriber) must mark the row delivered"
    );
    // The AcceptAll we created but did NOT register should have seen nothing.
    assert_eq!(accept.delivered(), 0, "unregistered subscriber receives nothing");
}

/// The claim loop is safe when no rows are due: the dispatcher polls, finds
/// nothing, waits, and shuts down cleanly.  Publishes a row with a far-future
/// `next_attempt_at` so it is intentionally NOT due.
#[tokio::test]
async fn dispatcher_shuts_down_cleanly_when_no_due_rows() {
    let Some(db) = test_db().await else { return };
    let _serial = SERIAL.lock().await;
    let topic = unique_topic("idle");

    // Row is due 1 hour from now — should not be claimed.
    db.execute(Statement::from_sql_and_values(
        DbBackend::MySql,
        "INSERT INTO `event_outbox` \
           (`public_id`, `topic`, `payload`, `status`, `next_attempt_at`, `created_at`) \
         VALUES (?, ?, ?, 'pending', NOW() + INTERVAL 3600 SECOND, NOW())",
        [
            Value::from(EventId::new().as_string()),
            Value::from(topic.clone()),
            Value::from(serde_json::json!({"future": true})),
        ],
    ))
    .await
    .expect("insert future row");

    let dispatcher = Arc::new(EventDispatcher::new(
        db.clone(),
        vec![Arc::new(AcceptAll::new()) as Arc<dyn Subscriber>],
        "test-idle",
    ));
    let handle = dispatcher.spawn();

    // Let the dispatcher poll one idle cycle, then shut down.
    tokio::time::sleep(Duration::from_millis(700)).await;
    handle.shutdown().await;

    // Row must still be pending (not claimed).
    let (status, attempts, _, _) = read_row(&db, &topic)
        .await
        .expect("future row must still exist");
    cleanup(&db, &topic).await;

    assert_eq!(status, "pending", "future row must not be claimed");
    assert_eq!(attempts, 0, "future row has no attempts");
}
