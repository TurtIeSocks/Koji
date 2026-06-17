//! sea-orm entity for the `event_outbox` table.
//!
//! Mirrors the DDL in
//! `crates/migration/src/m20260529_000002_create_event_tables.rs`
//! (events design spec, "Schema") exactly:
//!
//! - `id` BIGINT UNSIGNED AUTO_INCREMENT PK → `u64`
//! - `public_id` CHAR(26) UNIQUE (ULID) → `String`
//! - `payload` JSON → `Json` (`serde_json::Value`)
//! - `status` ENUM → `String` (the lower-case DB enum value; the dispatcher
//!   reads/writes the literal via raw SQL, so no typed enum is needed)
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
    pub status: String,
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // ---- Model serde (pure struct construction) ----------------------------

    #[test]
    fn model_serializes_all_fields() {
        let now = chrono::NaiveDateTime::parse_from_str("2026-06-01 12:00:00", "%Y-%m-%d %H:%M:%S")
            .unwrap();
        let m = Model {
            id: 1,
            public_id: "01ARZ3NDEKTSV4RRFFQ69G5FAV".to_string(),
            topic: "area.route_updated".to_string(),
            payload: json!({"area_id": 42}),
            status: "pending".to_string(),
            attempts: 0,
            max_attempts: 5,
            next_attempt_at: now,
            locked_by: None,
            lease_expires: None,
            last_error: None,
            created_at: now,
            delivered_at: None,
        };
        let v = serde_json::to_value(&m).expect("serialize model");
        assert_eq!(v["public_id"], "01ARZ3NDEKTSV4RRFFQ69G5FAV");
        assert_eq!(v["topic"], "area.route_updated");
        // `status` is the lower-case DB enum string stored verbatim.
        assert_eq!(v["status"], "pending");
        assert_eq!(v["attempts"], 0);
        assert_eq!(v["payload"]["area_id"], 42);
        assert!(v["locked_by"].is_null());
        assert!(v["last_error"].is_null());
    }

    #[test]
    fn model_with_optional_fields_serializes_correctly() {
        let now = chrono::NaiveDateTime::parse_from_str("2026-06-01 12:00:00", "%Y-%m-%d %H:%M:%S")
            .unwrap();
        let m = Model {
            id: 99,
            public_id: "01ARZ3NDEKTSV4RRFFQ69G5FAV".to_string(),
            topic: "t".to_string(),
            payload: json!(null),
            status: "dead".to_string(),
            attempts: 5,
            max_attempts: 5,
            next_attempt_at: now,
            locked_by: Some("worker-1".to_string()),
            lease_expires: Some(now),
            last_error: Some("connection refused".to_string()),
            created_at: now,
            delivered_at: None,
        };
        let v = serde_json::to_value(&m).expect("serialize");
        assert_eq!(v["status"], "dead");
        assert_eq!(v["locked_by"], "worker-1");
        assert_eq!(v["last_error"], "connection refused");
        assert!(v["delivered_at"].is_null());
    }

    #[test]
    fn model_with_delivered_at_serializes() {
        let now = chrono::NaiveDateTime::parse_from_str("2026-06-01 12:00:00", "%Y-%m-%d %H:%M:%S")
            .unwrap();
        let later =
            chrono::NaiveDateTime::parse_from_str("2026-06-01 12:00:05", "%Y-%m-%d %H:%M:%S")
                .unwrap();
        let m = Model {
            id: 7,
            public_id: "01ARZ3NDEKTSV4RRFFQ69G5FAV".to_string(),
            topic: "t".to_string(),
            payload: json!({}),
            status: "delivered".to_string(),
            attempts: 1,
            max_attempts: 5,
            next_attempt_at: now,
            locked_by: None,
            lease_expires: None,
            last_error: None,
            created_at: now,
            delivered_at: Some(later),
        };
        let v = serde_json::to_value(&m).expect("serialize");
        assert_eq!(v["status"], "delivered");
        assert!(!v["delivered_at"].is_null());
    }
}
