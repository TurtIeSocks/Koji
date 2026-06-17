//! sea-orm entity for the `job` table.
//!
//! Mirrors the DDL in `crates/migration/src/m20260529_000001_create_job_table.rs`
//! (job-queue design spec §4) exactly:
//!
//! - `id` BIGINT UNSIGNED AUTO_INCREMENT PK → `u64`
//! - `public_id` CHAR(26) UNIQUE (ULID) → `String`
//! - `payload` / `result` JSON → `Json` (`serde_json::Value`)
//! - `status` ENUM → [`JobStatus`] (a `DeriveActiveEnum`)
//! - the DATETIME columns are MySQL `DATETIME` (no tz) → sea-orm `DateTime`
//!   (= `chrono::NaiveDateTime`).
//!
//! The hot paths (claim / `enqueue_or_attach`) use raw SQL `Statement`s (see
//! `queue.rs`) because they need MySQL-specific clauses (`FOR UPDATE SKIP
//! LOCKED`, `NOW() + INTERVAL`). This entity backs the typed *reads* (`get`,
//! terminal-state short-circuit) and gives us a single source of truth for the
//! column/enum names.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

// No `Eq`: the `progress` column is `f32` (MySQL FLOAT), which is only `PartialEq`.
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "job")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: u64,
    #[sea_orm(unique)]
    pub public_id: String,
    pub kind: String,
    pub dedup_key: Option<String>,
    pub status: JobStatus,
    pub priority: i16,
    pub payload: Json,
    pub result: Option<Json>,
    #[sea_orm(column_type = "Text", nullable)]
    pub error: Option<String>,
    pub progress: f32,
    pub phase: Option<String>,
    pub attempts: i32,
    pub max_attempts: i32,
    pub locked_by: Option<String>,
    pub lease_expires: Option<DateTime>,
    pub created_at: DateTime,
    pub started_at: Option<DateTime>,
    pub finished_at: Option<DateTime>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

/// The `status` column ENUM: `'queued' | 'running' | 'succeeded' | 'failed' | 'canceled'`.
///
/// Stored as the lower-case string values from the DDL. `succeeded`, `failed`,
/// and `canceled` are the three terminal states.
#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "status")]
pub enum JobStatus {
    #[sea_orm(string_value = "queued")]
    Queued,
    #[sea_orm(string_value = "running")]
    Running,
    #[sea_orm(string_value = "succeeded")]
    Succeeded,
    #[sea_orm(string_value = "failed")]
    Failed,
    #[sea_orm(string_value = "canceled")]
    Canceled,
}

impl JobStatus {
    /// `true` for `succeeded` / `failed` / `canceled` — states that will never
    /// change again, so `await_result` can short-circuit on them.
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            JobStatus::Succeeded | JobStatus::Failed | JobStatus::Canceled
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── JobStatus::is_terminal ─────────────────────────────────────────────

    #[test]
    fn terminal_states_are_exactly_succeeded_failed_canceled() {
        assert!(JobStatus::Succeeded.is_terminal());
        assert!(JobStatus::Failed.is_terminal());
        assert!(JobStatus::Canceled.is_terminal());
        assert!(!JobStatus::Queued.is_terminal());
        assert!(!JobStatus::Running.is_terminal());
    }

    // ── serde round-trip ──────────────────────────────────────────────────

    #[test]
    fn job_status_serde_round_trips() {
        let variants = [
            JobStatus::Queued,
            JobStatus::Running,
            JobStatus::Succeeded,
            JobStatus::Failed,
            JobStatus::Canceled,
        ];
        for v in variants {
            let json = serde_json::to_string(&v).expect("serialize");
            let back: JobStatus = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(back, v, "round-trip failed for {v:?}");
        }
    }

    /// serde's derived (De)Serialize uses Rust variant names (PascalCase), NOT
    /// the DB ENUM string values. The DB-string form (`as_str`) is only for raw
    /// SQL construction; the serde wire form is `"Queued"` / `"Running"` / etc.
    #[test]
    fn job_status_deserializes_from_rust_variant_names() {
        let cases = [
            (r#""Queued""#, JobStatus::Queued),
            (r#""Running""#, JobStatus::Running),
            (r#""Succeeded""#, JobStatus::Succeeded),
            (r#""Failed""#, JobStatus::Failed),
            (r#""Canceled""#, JobStatus::Canceled),
        ];
        for (json, expected) in cases {
            let got: JobStatus = serde_json::from_str(json).expect(json);
            assert_eq!(got, expected);
        }
    }

    /// Document the serde/DB split: lowercase DB strings are NOT the serde form.
    ///
    /// The DB ENUM uses lower-case string values (raw SQL); serde uses PascalCase
    /// variant names. If the API ever serializes a `JobStatus` to JSON and
    /// round-trips it, the consumer must use `"Queued"` not `"queued"`.
    #[test]
    fn job_status_db_strings_are_not_serde_strings() {
        assert!(serde_json::from_str::<JobStatus>(r#""queued""#).is_err());
        assert!(serde_json::from_str::<JobStatus>(r#""running""#).is_err());
        assert!(serde_json::from_str::<JobStatus>(r#""succeeded""#).is_err());
    }

    #[test]
    fn job_status_rejects_unknown_string() {
        let result = serde_json::from_str::<JobStatus>(r#""cancelled""#); // British spelling
        assert!(
            result.is_err(),
            "unknown variant should fail to deserialize"
        );
    }

    // ── Clone / Copy / PartialEq ───────────────────────────────────────────

    #[test]
    fn job_status_copy_clone_eq() {
        let a = JobStatus::Running;
        let b = a; // Copy
        #[allow(clippy::clone_on_copy)]
        let c = a.clone(); // Clone — intentionally exercising the Clone impl
        assert_eq!(a, b);
        assert_eq!(a, c);
        assert_ne!(a, JobStatus::Queued);
    }

    // ── Model field coverage — pure struct construction ────────────────────

    #[test]
    fn model_fields_are_accessible() {
        // Construct a minimal Model to verify the field names / types match the
        // entity definition without hitting the DB.
        let now = chrono::DateTime::from_timestamp(0, 0).unwrap().naive_utc();
        let model = Model {
            id: 1,
            public_id: "01HZZZZZZZZZZZZZZZZZZZZZZ".to_string(),
            kind: "calc.route".to_string(),
            dedup_key: Some("abc123".to_string()),
            status: JobStatus::Queued,
            priority: 0,
            payload: serde_json::json!({"radius": 70}),
            result: None,
            error: None,
            progress: 0.0,
            phase: None,
            attempts: 0,
            max_attempts: 3,
            locked_by: None,
            lease_expires: None,
            created_at: now,
            started_at: None,
            finished_at: None,
        };

        assert_eq!(model.id, 1);
        assert_eq!(model.kind, "calc.route");
        assert_eq!(model.status, JobStatus::Queued);
        assert!(!model.status.is_terminal());
        assert_eq!(model.dedup_key.as_deref(), Some("abc123"));
        assert_eq!(model.max_attempts, 3);
        assert!((model.progress - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn model_with_terminal_status_and_result() {
        let now = chrono::DateTime::from_timestamp(1_000_000, 0)
            .unwrap()
            .naive_utc();
        let result_json = serde_json::json!({"routes": 5, "status": "ok"});
        let model = Model {
            id: 42,
            public_id: "01HZZZZZZZZZZZZZZZZZZZZZZ".to_string(),
            kind: "calc.route".to_string(),
            dedup_key: None,
            status: JobStatus::Succeeded,
            priority: 1,
            payload: serde_json::json!({}),
            result: Some(result_json.clone()),
            error: None,
            progress: 1.0,
            phase: Some("done".to_string()),
            attempts: 1,
            max_attempts: 3,
            locked_by: None,
            lease_expires: None,
            created_at: now,
            started_at: Some(now),
            finished_at: Some(now),
        };

        assert!(model.status.is_terminal());
        assert_eq!(model.result, Some(result_json));
        assert_eq!(model.phase.as_deref(), Some("done"));
    }

    #[test]
    fn model_failed_state_carries_error_text() {
        let now = chrono::DateTime::from_timestamp(0, 0).unwrap().naive_utc();
        let model = Model {
            id: 7,
            public_id: "01HZZZZZZZZZZZZZZZZZZZZZZ".to_string(),
            kind: "calc.bootstrap".to_string(),
            dedup_key: None,
            status: JobStatus::Failed,
            priority: 0,
            payload: serde_json::json!({}),
            result: None,
            error: Some("validation error: radius must be > 0".to_string()),
            progress: 0.0,
            phase: None,
            attempts: 3,
            max_attempts: 3,
            locked_by: None,
            lease_expires: None,
            created_at: now,
            started_at: Some(now),
            finished_at: Some(now),
        };

        assert!(model.status.is_terminal());
        assert_eq!(
            model.error.as_deref(),
            Some("validation error: radius must be > 0")
        );
        assert!(model.result.is_none());
    }
}
