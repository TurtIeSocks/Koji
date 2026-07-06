//! sea-orm entity for the `webhook_subscription` table.
//!
//! Mirrors the DDL in
//! `crates/migration/src/m20260529_000002_create_event_tables.rs`
//! (events design spec, "Schema") exactly:
//!
//! - `id` BIGINT UNSIGNED AUTO_INCREMENT PK → `u64`
//! - `url` VARCHAR(512) → `String`
//! - `secret` VARCHAR(255) NULL → `Option<String>` (HMAC-SHA256 signing key)
//! - `topics` JSON → `Json` (a JSON array of topic strings; empty = all)
//! - `active` TINYINT(1) → `bool`
//! - `name` VARCHAR(255) → `String` (admin-UI label)
//! - `project_id` INT UNSIGNED NULL → `Option<u32>` (FK → project; NULL = global)
//! - `mode` ENUM('event','ping') → `String` ("event" = signed JSON POST, "ping" = legacy reload)
//! - `method` ENUM('GET','POST') → `String` (ping mode only)
//! - `headers` JSON NULL → `Option<Json>` (custom header map, both modes)
//! - `created_at` / `updated_at` DATETIME → sea-orm `DateTime`.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "webhook_subscription")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: u64,
    pub url: String,
    pub secret: Option<String>,
    pub topics: Json,
    pub active: bool,
    pub name: String,
    pub project_id: Option<u32>,
    pub mode: String,
    pub method: String,
    pub headers: Option<Json>,
    pub created_at: DateTime,
    pub updated_at: DateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
