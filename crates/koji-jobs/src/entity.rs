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
    /// The DB string value (matches the ENUM definition).
    pub fn as_str(self) -> &'static str {
        match self {
            JobStatus::Queued => "queued",
            JobStatus::Running => "running",
            JobStatus::Succeeded => "succeeded",
            JobStatus::Failed => "failed",
            JobStatus::Canceled => "canceled",
        }
    }

    /// `true` for `succeeded` / `failed` / `canceled` — states that will never
    /// change again, so `await_result` can short-circuit on them.
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            JobStatus::Succeeded | JobStatus::Failed | JobStatus::Canceled
        )
    }
}
