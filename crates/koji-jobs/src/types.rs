//! Public value types for the job queue (design spec §10).
//!
//! - [`JobId`] — the opaque, sortable, API-facing identifier (`public_id`,
//!   ULID-backed).
//! - [`JobOutcome`] — the terminal result handed to waiters: success JSON or a
//!   failure (error message + stable code).
//! - [`JobRecord`] — a read-only view of a job's observable state.
//! - [`CancelToken`] — cooperative cancellation flag (honored at phase
//!   boundaries; rayon work can't be interrupted mid-pass — spec §2 non-goals).
//! - [`ProgressHandle`] — lets a handler persist coarse `progress`/`phase`.
//! - [`JobCtx`] — the `{cancel, progress}` bundle passed to a handler's `run`.

use std::fmt;
use std::str::FromStr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, Statement, Value};
use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error as _};
use ulid::Ulid;

use crate::entity::JobStatus;

/// Opaque, sortable, API-facing job identifier — the `public_id` column.
///
/// Backed by a ULID (Crockford base32, 26 chars): lexicographically sortable by
/// creation time and safe to expose to clients (unlike the auto-increment
/// `BIGINT` PK). `Display`/`FromStr` round-trip through the canonical ULID
/// string, and serde (de)serializes it as that same 26-char string — so the
/// JSON wire form is the opaque id clients see, never the raw u128.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct JobId(pub Ulid);

impl Serialize for JobId {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        // The canonical ULID string is the API-facing representation.
        serializer.serialize_str(&self.0.to_string())
    }
}

impl<'de> Deserialize<'de> for JobId {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Ulid::from_string(&s)
            .map(JobId)
            .map_err(|e| D::Error::custom(format!("invalid ULID: {e}")))
    }
}

impl JobId {
    /// Generate a fresh, time-ordered id.
    pub fn new() -> Self {
        JobId(Ulid::new())
    }

    /// The canonical 26-char ULID string (this is exactly what is stored in
    /// `public_id`).
    pub fn as_string(&self) -> String {
        self.0.to_string()
    }
}

impl Default for JobId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for JobId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // `Ulid`'s Display writes the canonical 26-char encoding.
        write!(f, "{}", self.0)
    }
}

impl FromStr for JobId {
    type Err = ulid::DecodeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(JobId(Ulid::from_string(s)?))
    }
}

/// The terminal result of a job, delivered to in-process waiters and reconstructable
/// from a terminal DB row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JobOutcome {
    /// `status='succeeded'`: the handler's result JSON.
    Succeeded(serde_json::Value),
    /// `status='failed'`: the persisted error message + its stable v2 code.
    Failed {
        /// `job.error` text.
        error: String,
        /// Stable v2 error code (e.g. `validation_error`).
        code: String,
    },
    /// `status='canceled'`: the job was canceled before completing.
    Canceled,
}

/// Read-only view of a job's observable state (returned by
/// [`crate::JobQueue::get`]).
///
/// Intentionally excludes internal claim bookkeeping (`locked_by`,
/// `lease_expires`, `attempts`) — those are runtime concerns, not part of the
/// public surface.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobRecord {
    /// API-facing id.
    pub id: JobId,
    /// Handler kind that owns this job.
    pub kind: String,
    /// Current lifecycle state.
    pub status: JobStatus,
    /// Coarse progress in `[0.0, 1.0]`.
    pub progress: f32,
    /// Coarse phase marker (`queued`→`clustering`→`routing`→`done`), if set.
    pub phase: Option<String>,
    /// Result JSON, present once `succeeded`.
    pub result: Option<serde_json::Value>,
    /// Error text, present once `failed`.
    pub error: Option<String>,
}

/// Cooperative cancellation flag shared between [`JobCtx`] and
/// [`crate::JobQueue::cancel`].
///
/// `cancel()` flips an atomic bool; a handler polls [`CancelToken::is_cancelled`]
/// at phase boundaries and bails out early (spec §9: "honored at the next phase
/// boundary"). It is *not* a hard interrupt — in-flight rayon work runs to the
/// next checkpoint.
#[derive(Debug, Clone, Default)]
pub struct CancelToken(Arc<AtomicBool>);

impl CancelToken {
    /// A fresh, un-cancelled token.
    pub fn new() -> Self {
        CancelToken(Arc::new(AtomicBool::new(false)))
    }

    /// Request cancellation. Idempotent.
    pub fn cancel(&self) {
        self.0.store(true, Ordering::SeqCst);
    }

    /// Whether cancellation has been requested.
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}

/// Handle a running handler uses to publish coarse progress + phase back to the
/// `job` row.
///
/// Kept deliberately simple (spec §10): it holds the job's numeric id and a DB
/// connection handle, and `set` issues a single targeted `UPDATE`. Progress is
/// coarse (phase markers), so the write rate is low — no batching/channel needed.
#[derive(Clone)]
pub struct ProgressHandle {
    db: DatabaseConnection,
    /// The numeric PK (`job.id`) — internal, never exposed to clients.
    job_id: u64,
}

impl ProgressHandle {
    /// Build a handle bound to a specific job row.
    pub(crate) fn new(db: DatabaseConnection, job_id: u64) -> Self {
        ProgressHandle { db, job_id }
    }

    /// Persist `progress` (clamped to `[0.0, 1.0]`) and an optional `phase`.
    ///
    /// Best-effort relative to the job itself: a progress write that fails is
    /// logged but does not fail the job (cosmetic state only).
    pub async fn set(&self, progress: f32, phase: Option<&str>) {
        let progress = progress.clamp(0.0, 1.0);
        let stmt = Statement::from_sql_and_values(
            DbBackend::MySql,
            "UPDATE `job` SET `progress` = ?, `phase` = ? WHERE `id` = ?",
            [
                Value::from(progress),
                match phase {
                    Some(p) => Value::from(p.to_owned()),
                    None => Value::from(Option::<String>::None),
                },
                Value::from(self.job_id),
            ],
        );
        if let Err(e) = self.db.execute(stmt).await {
            log::warn!(
                "[koji-jobs] failed to persist progress for job id={}: {e}",
                self.job_id
            );
        }
    }
}

impl fmt::Debug for ProgressHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ProgressHandle")
            .field("job_id", &self.job_id)
            .finish_non_exhaustive()
    }
}

/// The execution context handed to [`crate::JobHandler::run`] (spec §10).
///
/// Bundles the cancellation flag and the progress sink so a handler can react to
/// cancellation and report coarse progress without knowing anything about the
/// queue internals.
#[derive(Debug, Clone)]
pub struct JobCtx {
    /// Cooperative cancellation flag, checked at phase boundaries.
    pub cancel: CancelToken,
    /// Coarse progress/phase sink.
    pub progress: ProgressHandle,
}

impl JobCtx {
    /// Assemble a context from its parts.
    pub(crate) fn new(cancel: CancelToken, progress: ProgressHandle) -> Self {
        JobCtx { cancel, progress }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::str::FromStr;

    // ── JobId ─────────────────────────────────────────────────────────────

    #[test]
    fn job_id_new_generates_26_char_ulid() {
        let id = JobId::new();
        assert_eq!(id.to_string().len(), 26);
    }

    #[test]
    fn job_id_display_and_from_str_round_trip() {
        let id = JobId::new();
        let s = id.to_string();
        let parsed = JobId::from_str(&s).expect("valid ULID must parse");
        assert_eq!(parsed, id);
    }

    #[test]
    fn job_id_as_string_matches_display() {
        let id = JobId::new();
        assert_eq!(id.as_string(), id.to_string());
    }

    #[test]
    fn job_id_from_str_rejects_malformed() {
        assert!(JobId::from_str("not-a-ulid").is_err());
        assert!(JobId::from_str("").is_err());
        assert!(JobId::from_str("01HZZZ").is_err()); // too short
    }

    #[test]
    fn job_id_serde_round_trips_as_ulid_string() {
        let id = JobId::new();
        let json = serde_json::to_string(&id).expect("serialize");
        // Wire form is a quoted 26-char ULID string.
        assert_eq!(json.len(), 28); // 26 + 2 quotes
        let back: JobId = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, id);
    }

    #[test]
    fn job_id_deserialize_rejects_invalid_string() {
        let result = serde_json::from_str::<JobId>(r#""not-a-ulid""#);
        assert!(result.is_err(), "invalid ULID string must fail to deserialize");
    }

    #[test]
    fn job_id_hash_and_eq() {
        use std::collections::HashSet;
        let id = JobId::new();
        let mut set = HashSet::new();
        set.insert(id);
        assert!(set.contains(&id));
        // A different id must not be in the set.
        assert!(!set.contains(&JobId::new()));
    }

    #[test]
    fn two_distinct_job_ids_are_not_equal() {
        // ULID time component ensures uniqueness at millisecond granularity;
        // the random component provides additional entropy.
        let a = JobId::new();
        let b = JobId::new();
        // In the vanishingly unlikely event they collide in the same ms, this
        // test may be flaky — acceptable given the 80-bit random component.
        assert_ne!(a, b);
    }

    #[test]
    fn job_id_default_generates_a_valid_id() {
        let id = JobId::default();
        assert_eq!(id.to_string().len(), 26);
    }

    #[test]
    fn job_id_copy_clone_eq() {
        let id = JobId::new();
        let copy = id; // Copy
        let clone = id.clone();
        assert_eq!(id, copy);
        assert_eq!(id, clone);
    }

    // ── JobOutcome ────────────────────────────────────────────────────────

    #[test]
    fn job_outcome_succeeded_holds_json() {
        let v = json!({"count": 42});
        let outcome = JobOutcome::Succeeded(v.clone());
        match outcome {
            JobOutcome::Succeeded(got) => assert_eq!(got, v),
            _ => panic!("expected Succeeded"),
        }
    }

    #[test]
    fn job_outcome_failed_holds_error_and_code() {
        let outcome = JobOutcome::Failed {
            error: "bad payload".to_string(),
            code: "validation_error".to_string(),
        };
        match outcome {
            JobOutcome::Failed { error, code } => {
                assert_eq!(error, "bad payload");
                assert_eq!(code, "validation_error");
            }
            _ => panic!("expected Failed"),
        }
    }

    #[test]
    fn job_outcome_canceled_variant() {
        let outcome = JobOutcome::Canceled;
        assert!(matches!(outcome, JobOutcome::Canceled));
    }

    #[test]
    fn job_outcome_eq() {
        let a = JobOutcome::Succeeded(json!(1));
        let b = JobOutcome::Succeeded(json!(1));
        let c = JobOutcome::Succeeded(json!(2));
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert_ne!(a, JobOutcome::Canceled);
        let f1 = JobOutcome::Failed { error: "e".to_string(), code: "c".to_string() };
        let f2 = JobOutcome::Failed { error: "e".to_string(), code: "c".to_string() };
        assert_eq!(f1, f2);
    }

    #[test]
    fn job_outcome_clone() {
        let original = JobOutcome::Failed {
            error: "oops".to_string(),
            code: "internal_error".to_string(),
        };
        let cloned = original.clone();
        assert_eq!(original, cloned);
    }

    // ── CancelToken ───────────────────────────────────────────────────────

    #[test]
    fn cancel_token_starts_not_cancelled() {
        let t = CancelToken::new();
        assert!(!t.is_cancelled());
    }

    #[test]
    fn cancel_token_cancel_flips_flag() {
        let t = CancelToken::new();
        t.cancel();
        assert!(t.is_cancelled());
    }

    #[test]
    fn cancel_token_cancel_is_idempotent() {
        let t = CancelToken::new();
        t.cancel();
        t.cancel(); // second call must not panic or reset
        assert!(t.is_cancelled());
    }

    #[test]
    fn cancel_token_clone_shares_flag() {
        let t = CancelToken::new();
        let clone = t.clone();
        // Initially both false.
        assert!(!t.is_cancelled());
        assert!(!clone.is_cancelled());
        // Cancel via original; clone sees it.
        t.cancel();
        assert!(clone.is_cancelled());
    }

    #[test]
    fn cancel_token_clone_cancel_visible_on_original() {
        // Symmetry: cancelling via the clone is visible on the original.
        let t = CancelToken::new();
        let clone = t.clone();
        clone.cancel();
        assert!(t.is_cancelled());
    }

    #[test]
    fn cancel_token_default_is_new() {
        let t = CancelToken::default();
        assert!(!t.is_cancelled());
    }

    #[test]
    fn cancel_token_debug() {
        let t = CancelToken::new();
        let dbg = format!("{t:?}");
        assert!(dbg.contains("CancelToken") || dbg.contains("false"));
    }

    // ── JobRecord serde round-trip ────────────────────────────────────────

    #[test]
    fn job_record_serde_round_trips() {
        use crate::entity::JobStatus;
        let id = JobId::new();
        let record = JobRecord {
            id,
            kind: "calc.route".to_string(),
            status: JobStatus::Running,
            progress: 0.42,
            phase: Some("clustering".to_string()),
            result: None,
            error: None,
        };
        let json = serde_json::to_string(&record).expect("serialize");
        let back: JobRecord = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.id, id);
        assert_eq!(back.kind, "calc.route");
        assert_eq!(back.status, JobStatus::Running);
        // f32 round-trip through JSON may have tiny epsilon; use approx check.
        assert!((back.progress - 0.42_f32).abs() < 0.001);
        assert_eq!(back.phase.as_deref(), Some("clustering"));
        assert!(back.result.is_none());
    }

    #[test]
    fn job_record_with_result_round_trips() {
        use crate::entity::JobStatus;
        let id = JobId::new();
        let result_val = json!({"routes": 5});
        let record = JobRecord {
            id,
            kind: "calc.route".to_string(),
            status: JobStatus::Succeeded,
            progress: 1.0,
            phase: None,
            result: Some(result_val.clone()),
            error: None,
        };
        let json = serde_json::to_string(&record).expect("serialize");
        let back: JobRecord = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.result, Some(result_val));
        assert!(back.error.is_none());
    }

    #[test]
    fn job_record_with_error_round_trips() {
        use crate::entity::JobStatus;
        let id = JobId::new();
        let record = JobRecord {
            id,
            kind: "calc.bootstrap".to_string(),
            status: JobStatus::Failed,
            progress: 0.0,
            phase: None,
            result: None,
            error: Some("validation error: radius must be > 0".to_string()),
        };
        let json = serde_json::to_string(&record).expect("serialize");
        let back: JobRecord = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.error.as_deref(), Some("validation error: radius must be > 0"));
        assert!(back.result.is_none());
        assert_eq!(back.status, JobStatus::Failed);
    }
}
