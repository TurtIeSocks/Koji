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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // ---- EventStatus::as_str ------------------------------------------------

    #[test]
    fn as_str_matches_db_enum_values() {
        assert_eq!(EventStatus::Pending.as_str(), "pending");
        assert_eq!(EventStatus::Delivering.as_str(), "delivering");
        assert_eq!(EventStatus::Delivered.as_str(), "delivered");
        assert_eq!(EventStatus::Dead.as_str(), "dead");
    }

    // ---- EventStatus::is_terminal -------------------------------------------

    #[test]
    fn delivered_is_terminal() {
        assert!(EventStatus::Delivered.is_terminal());
    }

    #[test]
    fn dead_is_terminal() {
        assert!(EventStatus::Dead.is_terminal());
    }

    #[test]
    fn pending_is_not_terminal() {
        assert!(!EventStatus::Pending.is_terminal());
    }

    #[test]
    fn delivering_is_not_terminal() {
        assert!(!EventStatus::Delivering.is_terminal());
    }

    // ---- EventStatus copy / clone / eq -------------------------------------

    #[test]
    fn event_status_copy_semantics() {
        let s = EventStatus::Pending;
        let t = s; // copy
        assert_eq!(s, t);
    }

    #[test]
    fn event_status_all_variants_distinct() {
        let variants = [
            EventStatus::Pending,
            EventStatus::Delivering,
            EventStatus::Delivered,
            EventStatus::Dead,
        ];
        for (i, a) in variants.iter().enumerate() {
            for (j, b) in variants.iter().enumerate() {
                if i == j {
                    assert_eq!(a, b);
                } else {
                    assert_ne!(a, b);
                }
            }
        }
    }

    // ---- EventStatus serde --------------------------------------------------
    //
    // `DeriveActiveEnum` delegates to serde's default derive, which serializes
    // Rust variant names as PascalCase strings — "Pending", "Delivering", etc.
    // The *DB* representation ("pending", "delivering" …) is separate: that is
    // the sea-orm `string_value` attribute accessed via `as_str()`.  The two
    // must NOT be conflated.

    #[test]
    fn event_status_serializes_as_pascal_case() {
        assert_eq!(
            serde_json::to_string(&EventStatus::Pending).unwrap(),
            r#""Pending""#
        );
        assert_eq!(
            serde_json::to_string(&EventStatus::Delivering).unwrap(),
            r#""Delivering""#
        );
        assert_eq!(
            serde_json::to_string(&EventStatus::Delivered).unwrap(),
            r#""Delivered""#
        );
        assert_eq!(
            serde_json::to_string(&EventStatus::Dead).unwrap(),
            r#""Dead""#
        );
    }

    #[test]
    fn event_status_deserializes_from_pascal_case() {
        let s: EventStatus = serde_json::from_str(r#""Pending""#).unwrap();
        assert_eq!(s, EventStatus::Pending);
        let s: EventStatus = serde_json::from_str(r#""Delivering""#).unwrap();
        assert_eq!(s, EventStatus::Delivering);
        let s: EventStatus = serde_json::from_str(r#""Delivered""#).unwrap();
        assert_eq!(s, EventStatus::Delivered);
        let s: EventStatus = serde_json::from_str(r#""Dead""#).unwrap();
        assert_eq!(s, EventStatus::Dead);
    }

    /// as_str() is the DB ENUM value (lowercase); serde is PascalCase.
    /// These MUST differ — they serve different serialization surfaces.
    #[test]
    fn as_str_and_serde_json_value_are_different_representations() {
        // as_str → lowercase (sea-orm DB enum string)
        assert_eq!(EventStatus::Pending.as_str(), "pending");
        // serde_json → PascalCase (Rust variant name)
        let json_repr = serde_json::to_string(&EventStatus::Pending).unwrap();
        assert_eq!(json_repr, r#""Pending""#);
        assert_ne!(json_repr.trim_matches('"'), EventStatus::Pending.as_str());
    }

    // ---- Model serde (pure struct construction) ----------------------------

    #[test]
    fn model_serializes_all_fields() {
        let now = chrono::NaiveDateTime::parse_from_str(
            "2026-06-01 12:00:00",
            "%Y-%m-%d %H:%M:%S",
        )
        .unwrap();
        let m = Model {
            id: 1,
            public_id: "01ARZ3NDEKTSV4RRFFQ69G5FAV".to_string(),
            topic: "area.route_updated".to_string(),
            payload: json!({"area_id": 42}),
            status: EventStatus::Pending,
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
        // serde derives PascalCase for enum variants; "Pending" not "pending"
        assert_eq!(v["status"], "Pending");
        assert_eq!(v["attempts"], 0);
        assert_eq!(v["payload"]["area_id"], 42);
        assert!(v["locked_by"].is_null());
        assert!(v["last_error"].is_null());
    }

    #[test]
    fn model_with_optional_fields_serializes_correctly() {
        let now = chrono::NaiveDateTime::parse_from_str(
            "2026-06-01 12:00:00",
            "%Y-%m-%d %H:%M:%S",
        )
        .unwrap();
        let m = Model {
            id: 99,
            public_id: "01ARZ3NDEKTSV4RRFFQ69G5FAV".to_string(),
            topic: "t".to_string(),
            payload: json!(null),
            status: EventStatus::Dead,
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
        assert_eq!(v["status"], "Dead"); // PascalCase via serde derive
        assert_eq!(v["locked_by"], "worker-1");
        assert_eq!(v["last_error"], "connection refused");
        assert!(v["delivered_at"].is_null());
    }

    #[test]
    fn model_with_delivered_at_serializes() {
        let now = chrono::NaiveDateTime::parse_from_str(
            "2026-06-01 12:00:00",
            "%Y-%m-%d %H:%M:%S",
        )
        .unwrap();
        let later = chrono::NaiveDateTime::parse_from_str(
            "2026-06-01 12:00:05",
            "%Y-%m-%d %H:%M:%S",
        )
        .unwrap();
        let m = Model {
            id: 7,
            public_id: "01ARZ3NDEKTSV4RRFFQ69G5FAV".to_string(),
            topic: "t".to_string(),
            payload: json!({}),
            status: EventStatus::Delivered,
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
        assert_eq!(v["status"], "Delivered"); // PascalCase
        assert!(!v["delivered_at"].is_null());
    }
}
