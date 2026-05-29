//! sea-orm entity for the `event_outbox` table.
//!
//! Mirrors the DDL in
//! `crates/migration/src/m20260529_000002_create_event_tables.rs`
//! (events design spec, "Schema") exactly:
//!
//! - `id` BIGINT UNSIGNED AUTO_INCREMENT PK → `u64`
//! - `public_id` CHAR(26) UNIQUE (ULID) → `String`
//! - `payload` JSON → `Json` (`serde_json::Value`)
//! - `status` ENUM → [`EventStatus`] (a `DeriveActiveEnum`)
//! - the DATETIME columns are MySQL `DATETIME` (no tz) → sea-orm `DateTime`
//!   (= `chrono::NaiveDateTime`).
//!
//! The hot paths (claim / backoff) use raw SQL `Statement`s (see
//! `dispatcher.rs`) because they need MySQL-specific clauses (`FOR UPDATE SKIP
//! LOCKED`, `NOW() + INTERVAL`). This entity is the single source of truth for
//! the column/enum names.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "event_outbox")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: u64,
    #[sea_orm(unique)]
    pub public_id: String,
    pub topic: String,
    pub payload: Json,
    pub status: EventStatus,
    pub attempts: i32,
    pub max_attempts: i32,
    pub next_attempt_at: DateTime,
    pub locked_by: Option<String>,
    pub lease_expires: Option<DateTime>,
    #[sea_orm(column_type = "Text", nullable)]
    pub last_error: Option<String>,
    pub created_at: DateTime,
    pub delivered_at: Option<DateTime>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

/// The `status` column ENUM: `'pending' | 'delivering' | 'delivered' | 'dead'`.
///
/// Stored as the lower-case string values from the DDL. `delivered` and `dead`
/// are the two terminal states.
#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "status")]
pub enum EventStatus {
    #[sea_orm(string_value = "pending")]
    Pending,
    #[sea_orm(string_value = "delivering")]
    Delivering,
    #[sea_orm(string_value = "delivered")]
    Delivered,
    #[sea_orm(string_value = "dead")]
    Dead,
}

impl EventStatus {
    /// The DB string value (matches the ENUM definition).
    pub fn as_str(self) -> &'static str {
        match self {
            EventStatus::Pending => "pending",
            EventStatus::Delivering => "delivering",
            EventStatus::Delivered => "delivered",
            EventStatus::Dead => "dead",
        }
    }

    /// `true` for `delivered` / `dead` — states that will never change again.
    pub fn is_terminal(self) -> bool {
        matches!(self, EventStatus::Delivered | EventStatus::Dead)
    }
}
