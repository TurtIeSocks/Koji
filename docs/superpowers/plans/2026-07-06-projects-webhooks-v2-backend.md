# Projects v2 Webhook-Backed Push — Backend Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the v1 project push mechanism (`api_endpoint`/`api_key`/`golbat` columns) with project-scoped webhook subscriptions riding the existing koji-events outbox/dispatcher.

**Architecture:** `webhook_subscription` gains `name`/`project_id`/`mode`/`method`/`headers`; the dispatcher's `WebhookSubscriber` learns project-filtered matching and a legacy `ping` delivery mode; five new topics emit from service handlers; `/api/v2/webhooks` becomes the 6th `koji_resource!`; a data migration converts v1 push config into ping subscriptions, then a second migration drops the dead project columns.

**Tech Stack:** Rust (edition 2024), actix-web, sea-orm (MySQL), sea-orm-migration, reqwest, hmac/sha2 (existing koji-events deps — no new dependencies anywhere in this plan).

**Spec:** `docs/superpowers/specs/2026-07-06-projects-webhooks-v2-design.md`

## Global Constraints

- Branch: `claude/v2`. Commit after every task (project rule: commit freely, conventional style).
- MySQL 8+ / MariaDB 10.6+ assumed (existing `SKIP LOCKED` requirement).
- DB-touching tests: gate on `KOJI_DB_URL` with `let Some(db) = test_db().await else { return };` — never `#[ignore]`. Source env first: `set -a; source ./.env.test; set +a`.
- The server crate's package name is `koji` (NOT `koji-server`): `cargo build -p koji`. Never pipe build output through `| tail` (masks exit code).
- Wire format for the new resource is **snake_case** (`project_id`, not `projectId`) — matches every other `koji_resource!` resource. The list filter param is **`?project=`** (existing `ListQuery` convention). These are deliberate deviations from the spec's camelCase examples; Task 10 amends the spec.
- Event payload keys are camelCase (`projectId`, `projectIds`, `geofenceId`, `addedIds`, `removedIds`) — matches the existing dragonite payload style (`dragonite_area_id` is the exception, do not copy it).
- Outbox publish failures in request handlers are logged (`log::warn!`) and never fail the HTTP request — a missed ping is not data loss.
- No new crate dependencies. `koji-events` keeps depending only on `koji-core` (+ sea-orm/reqwest/etc.).

---

### Task 1: Migration A — extend `webhook_subscription` + migrate v1 push config

**Files:**
- Create: `crates/migration/src/m20260706_000001_webhook_subscription_projects.rs`
- Modify: `crates/migration/src/lib.rs` (register migration, add `mod` line)

**Interfaces:**
- Consumes: existing `project` table (`api_endpoint`, `api_key` columns still present — they are dropped later in Task 9).
- Produces: `webhook_subscription` columns `name VARCHAR(255) NOT NULL`, `project_id INT UNSIGNED NULL` (FK → `project.id` ON DELETE CASCADE), `mode ENUM('event','ping') NOT NULL DEFAULT 'event'`, `method ENUM('GET','POST') NOT NULL DEFAULT 'GET'`, `headers JSON NULL`. Ping-subscription rows for every project that had `api_endpoint` set. Later tasks (2, 3) mirror these columns in entities.

- [ ] **Step 1: Verify `project.id` column type** (the FK must match exactly)

Run: `set -a; source ./.env.test; set +a; mysql --protocol=tcp -h 127.0.0.1 -u koji_user -p"$(grep -oP '(?<=:\/\/koji_user:)[^@]+' <<<"$KOJI_DB_URL")" koji_database -e "SHOW CREATE TABLE project\G" 2>/dev/null | grep -i '`id`'`

Expected: `` `id` int unsigned NOT NULL AUTO_INCREMENT `` (or `int(10) unsigned` on MariaDB). If it is anything else, use that exact type for `project_id` below. (If the local DB is unreachable, proceed with `INT UNSIGNED` — the migration test in Step 5 will catch a mismatch.)

- [ ] **Step 2: Write the failing test** — the api_key parser is a pure function; test it first

Create `crates/migration/src/m20260706_000001_webhook_subscription_projects.rs` with ONLY the parser and its tests (migration struct comes in Step 4):

```rust
use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_orm::{ConnectionTrait, FromQueryResult, Statement};

#[derive(DeriveMigrationName)]
pub struct Migration;

/// Parse a v1 `project.api_key` into a webhook `headers` JSON map.
///
/// v1 format was `HeaderName:SecretValue` (split on the FIRST `:` — secrets may
/// contain colons). A bare token with no `:` becomes an `Authorization` header
/// so no working config is silently dropped. Empty/whitespace → `None`.
fn parse_api_key(raw: &str) -> Option<serde_json::Value> {
    let raw = raw.trim();
    if raw.is_empty() {
        return None;
    }
    match raw.split_once(':') {
        Some((name, value)) if !name.trim().is_empty() => {
            Some(serde_json::json!({ name.trim(): value.trim() }))
        }
        // ":secret" (empty header name) or bare token → Authorization.
        _ => Some(serde_json::json!({ "Authorization": raw })),
    }
}

#[cfg(test)]
mod tests {
    use super::parse_api_key;
    use serde_json::json;

    #[test]
    fn parses_header_colon_secret() {
        assert_eq!(
            parse_api_key("x-golbat-secret:abc123"),
            Some(json!({"x-golbat-secret": "abc123"}))
        );
    }

    #[test]
    fn splits_on_first_colon_only() {
        assert_eq!(
            parse_api_key("X-Poracle-Secret:se:cr:et"),
            Some(json!({"X-Poracle-Secret": "se:cr:et"}))
        );
    }

    #[test]
    fn bare_token_becomes_authorization() {
        assert_eq!(
            parse_api_key("Bearer abc"),
            Some(json!({"Authorization": "Bearer abc"}))
        );
    }

    #[test]
    fn empty_and_whitespace_are_none() {
        assert_eq!(parse_api_key(""), None);
        assert_eq!(parse_api_key("   "), None);
    }

    #[test]
    fn leading_colon_becomes_authorization() {
        assert_eq!(
            parse_api_key(":lonely"),
            Some(json!({"Authorization": ":lonely"}))
        );
    }

    #[test]
    fn trims_around_split() {
        assert_eq!(
            parse_api_key(" X-Key : v v "),
            Some(json!({"X-Key": "v v"}))
        );
    }
}
```

- [ ] **Step 3: Run test to verify it fails to compile** (module not registered yet), then register the module

Run: `cargo test -p migration parse_api_key`
Expected: FAIL — file not compiled. Add to `crates/migration/src/lib.rs`: `mod m20260706_000001_webhook_subscription_projects;` and append `Box::new(m20260706_000001_webhook_subscription_projects::Migration),` to the `migrations()` vec (after the `m20260620...` entry). Re-run — the parser tests must PASS (the not-yet-written `MigrationTrait` impl will fail the build; add it in Step 4 — you may write Steps 2–4 as one edit and rely on the test run in Step 5, keeping commits atomic per task).

- [ ] **Step 4: Write the migration body**

Append to the same file (idempotency-guard style copied from `m20260620_000001_rename_project_scanner_to_golbat.rs`):

```rust
/// True if `table` has a column named exactly `col`.
async fn has_column(manager: &SchemaManager<'_>, table: &str, col: &str) -> Result<bool, DbErr> {
    let conn = manager.get_connection();
    let stmt = Statement::from_sql_and_values(
        conn.get_database_backend(),
        "SELECT COLUMN_NAME FROM information_schema.COLUMNS \
         WHERE TABLE_SCHEMA = DATABASE() AND TABLE_NAME = ? AND COLUMN_NAME = ? LIMIT 1",
        [table.into(), col.into()],
    );
    Ok(conn.query_one(stmt).await?.is_some())
}

/// A v1 project row carrying push config, read before the drop in migration B.
#[derive(Debug, FromQueryResult)]
struct PushProject {
    id: u32,
    name: String,
    api_endpoint: Option<String>,
    api_key: Option<String>,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Projects-v2 push redesign (spec 2026-07-06): extend `webhook_subscription`
    /// with project scoping + legacy ping mode, then convert every project's v1
    /// push config (`api_endpoint`/`api_key`) into a ping subscription. The
    /// project columns themselves are dropped by the LATER migration
    /// `m20260706_000002` so this one can still read them. Guarded on column
    /// absence so re-runs and fresh DBs are no-ops for both DDL and data move.
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if has_column(manager, "webhook_subscription", "project_id").await? {
            log::info!("[MIGRATION] webhook_subscription already extended; skipping");
            return Ok(());
        }
        let db = manager.get_connection();
        log::info!("[MIGRATION] extending webhook_subscription with project push columns");
        db.execute_unprepared(
            r#"
ALTER TABLE `webhook_subscription`
  ADD COLUMN `name`       VARCHAR(255) NOT NULL DEFAULT '',
  ADD COLUMN `project_id` INT UNSIGNED NULL,
  ADD COLUMN `mode`       ENUM('event','ping') NOT NULL DEFAULT 'event',
  ADD COLUMN `method`     ENUM('GET','POST')   NOT NULL DEFAULT 'GET',
  ADD COLUMN `headers`    JSON NULL,
  ADD CONSTRAINT `fk_webhook_subscription_project`
    FOREIGN KEY (`project_id`) REFERENCES `project`(`id`) ON DELETE CASCADE;
            "#,
        )
        .await?;
        // Pre-existing rows (feature was unwired, so normally none): give them a
        // usable label anyway.
        db.execute_unprepared(
            "UPDATE `webhook_subscription` SET `name` = CONCAT('webhook-', `id`) WHERE `name` = ''",
        )
        .await?;

        // Data move: every project with a push endpoint becomes a ping
        // subscription with exact v1 semantics (GET, custom auth header, fire on
        // any change in the project = empty topics + project_id).
        // Skip when project.api_endpoint is already gone (fresh DB where
        // migration B's schema shipped from the start).
        if !has_column(manager, "project", "api_endpoint").await? {
            log::info!("[MIGRATION] project.api_endpoint absent; no v1 push config to migrate");
            return Ok(());
        }
        let rows = PushProject::find_by_statement(Statement::from_string(
            db.get_database_backend(),
            "SELECT `id`, `name`, `api_endpoint`, `api_key` FROM `project` \
             WHERE `api_endpoint` IS NOT NULL AND TRIM(`api_endpoint`) != ''",
        ))
        .all(db)
        .await?;
        log::info!("[MIGRATION] migrating {} project push config(s) to ping webhooks", rows.len());
        for row in rows {
            let headers = row.api_key.as_deref().and_then(parse_api_key);
            db.execute(Statement::from_sql_and_values(
                db.get_database_backend(),
                "INSERT INTO `webhook_subscription` \
                   (`name`, `url`, `secret`, `topics`, `active`, `project_id`, `mode`, `method`, `headers`, `created_at`, `updated_at`) \
                 VALUES (?, ?, NULL, '[]', 1, ?, 'ping', 'GET', ?, NOW(), NOW())",
                [
                    format!("{} reload", row.name).into(),
                    row.api_endpoint.unwrap_or_default().into(),
                    row.id.into(),
                    headers
                        .map(|h| h.to_string())
                        .map(sea_orm::Value::from)
                        .unwrap_or(sea_orm::Value::String(None)),
                ],
            ))
            .await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if !has_column(manager, "webhook_subscription", "project_id").await? {
            return Ok(());
        }
        // Data-move rows die with the columns; v1 config restore is migration
        // B's `down` concern (it re-adds the project columns, empty).
        manager
            .get_connection()
            .execute_unprepared(
                r#"
ALTER TABLE `webhook_subscription`
  DROP FOREIGN KEY `fk_webhook_subscription_project`,
  DROP COLUMN `project_id`,
  DROP COLUMN `name`,
  DROP COLUMN `mode`,
  DROP COLUMN `method`,
  DROP COLUMN `headers`;
                "#,
            )
            .await?;
        Ok(())
    }
}
```

- [ ] **Step 5: Run parser tests + apply the migration to the test DB**

Run (parallel-safe, one Bash batch):
- `cargo test -p migration parse_api_key` → Expected: 6 passed.
- `set -a; source ./.env.test; set +a; DATABASE_URL="$KOJI_DB_URL" cargo run -p migration -- up` → Expected: log lines `extending webhook_subscription...` then success. (Note: the migration binary reads `DATABASE_URL`, not `KOJI_DB_URL`.)
- Verify: `SHOW CREATE TABLE webhook_subscription` now shows the 5 columns + FK.

- [ ] **Step 6: Manual data-move smoke check** (one-off, against test DB)

Insert a fake v1 project row with `api_endpoint='http://x/reload'`, `api_key='x-golbat-secret:abc'`, run `-- down` then `-- up` for this migration (or fresh DB + full `up`), and `SELECT name, url, mode, method, headers, project_id FROM webhook_subscription` → expect the ping row with `headers = {"x-golbat-secret": "abc"}`. Delete the fake rows afterwards.

- [ ] **Step 7: Commit**

```bash
git add crates/migration
git commit -m "feat(migration): extend webhook_subscription for project push; migrate v1 api_endpoint/api_key to ping subscriptions"
```

---

### Task 2: koji-events — entity columns, project matching, ping delivery, `deliver_to`

**Files:**
- Modify: `crates/koji-events/src/entity/webhook_subscription.rs`
- Modify: `crates/koji-events/src/webhook.rs`
- Modify: `crates/koji-events/src/lib.rs` (only if `WebhookSubscriber` re-exports need additions — check; `deliver_to` is a method, likely nothing)

**Interfaces:**
- Consumes: Task 1 columns.
- Produces:
  - entity `Model` gains `pub name: String`, `pub project_id: Option<u32>`, `pub mode: String`, `pub method: String`, `pub headers: Option<Json>`.
  - `pub async fn WebhookSubscriber::deliver_to(&self, sub: &webhook_subscription::Model, event: &Event) -> Result<u16, DeliverError>` — delivers to ONE subscription (mode-aware), returns upstream HTTP status. Used by `deliver()` and by Task 5's `/test` endpoint.
  - `fn project_matches(sub_project_id: Option<u32>, payload: &serde_json::Value) -> bool` (private, unit-tested).
  - Delivery modes: `mode == "ping"` → `method` request, custom headers, empty body, no signature, still `X-Koji-Event-Id`; anything else (`"event"`) → existing signed-POST path + custom headers merged.

- [ ] **Step 1: Update the entity** (keep the DDL-mirror docblock in sync)

In `crates/koji-events/src/entity/webhook_subscription.rs`, extend the docblock and Model:

```rust
//! - `name` VARCHAR(255) → `String` (admin-UI label)
//! - `project_id` INT UNSIGNED NULL → `Option<u32>` (FK → project; NULL = global)
//! - `mode` ENUM('event','ping') → `String` ("event" = signed JSON POST, "ping" = legacy reload)
//! - `method` ENUM('GET','POST') → `String` (ping mode only)
//! - `headers` JSON NULL → `Option<Json>` (custom header map, both modes)
```

```rust
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "webhook_subscription")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: u64,
    pub name: String,
    pub url: String,
    pub secret: Option<String>,
    pub topics: Json,
    pub active: bool,
    pub project_id: Option<u32>,
    pub mode: String,
    pub method: String,
    pub headers: Option<Json>,
    pub created_at: DateTime,
    pub updated_at: DateTime,
}
```

(Keep `Relation`/`ActiveModelBehavior` as-is. Field order within the struct doesn't matter to sea-orm; keep new fields grouped after `active` for readability. `mode`/`method` stay plain `String` here — koji-events stays dependency-light; the typed enums live in koji-db (Task 3) where validation happens.)

- [ ] **Step 2: Write failing unit tests for `project_matches`** in the `webhook.rs` tests mod

```rust
#[test]
fn project_matches_none_is_global() {
    assert!(project_matches(None, &json!({"projectIds": [7]})));
    assert!(project_matches(None, &json!({})));
}

#[test]
fn project_matches_in_project_ids_array() {
    assert!(project_matches(Some(7), &json!({"projectIds": [3, 7]})));
    assert!(!project_matches(Some(8), &json!({"projectIds": [3, 7]})));
}

#[test]
fn project_matches_scalar_project_id() {
    assert!(project_matches(Some(7), &json!({"projectId": 7})));
    assert!(!project_matches(Some(7), &json!({"projectId": 8})));
}

#[test]
fn project_matches_no_project_keys_means_no_match_for_scoped_sub() {
    // A scoped subscription must NOT fire for events that carry no project info
    // (e.g. a future global topic) — scoping is opt-in per event.
    assert!(!project_matches(Some(7), &json!({"something": "else"})));
}
```

- [ ] **Step 3: Run tests to verify failure**

Run: `cargo test -p koji-events project_matches`
Expected: FAIL — `project_matches` not defined.

- [ ] **Step 4: Implement matching + mode-aware delivery**

In `crates/koji-events/src/webhook.rs`:

```rust
/// Whether a subscription's project scope matches the event payload.
///
/// `None` = global subscription → always true. `Some(pid)` matches when the
/// payload's `projectIds` array contains `pid`, or its scalar `projectId`
/// equals `pid`. Events with neither key never match a scoped subscription.
fn project_matches(sub_project_id: Option<u32>, payload: &serde_json::Value) -> bool {
    let Some(pid) = sub_project_id else { return true };
    let pid = pid as u64;
    if payload.get("projectId").and_then(serde_json::Value::as_u64) == Some(pid) {
        return true;
    }
    payload
        .get("projectIds")
        .and_then(serde_json::Value::as_array)
        .is_some_and(|arr| arr.iter().any(|v| v.as_u64() == Some(pid)))
}
```

Replace the body of `deliver` and add `deliver_to`:

```rust
impl WebhookSubscriber {
    /// Deliver `event` to ONE subscription, mode-aware. Returns the upstream
    /// HTTP status on success. Public so the service's `/test` endpoint can
    /// fire a single subscription outside the dispatcher loop.
    pub async fn deliver_to(
        &self,
        sub: &webhook_subscription::Model,
        event: &Event,
    ) -> Result<u16, DeliverError> {
        let mut req = if sub.mode == "ping" {
            // Legacy reload ping: consumer-chosen method, empty body, no HMAC.
            let method = if sub.method.eq_ignore_ascii_case("POST") {
                reqwest::Method::POST
            } else {
                reqwest::Method::GET
            };
            self.http.request(method, &sub.url)
        } else {
            // Signed event POST: HMAC over the exact bytes sent.
            let body = serde_json::to_string(&event.payload)
                .map_err(|e| DeliverError::Other(format!("serializing event payload: {e}")))?;
            let mut req = self
                .http
                .post(&sub.url)
                .header(reqwest::header::CONTENT_TYPE, "application/json");
            if let Some(secret) = sub.secret.as_deref() {
                req = req.header(SIGNATURE_HEADER, sign(secret, body.as_bytes()));
            }
            req.body(body)
        };
        req = req.header(EVENT_ID_HEADER, &event.id);
        // Custom headers apply in BOTH modes (ping auth, proxies in front of
        // event consumers). Non-string values are skipped, not stringified.
        if let Some(headers) = sub.headers.as_ref().and_then(|h| h.as_object()) {
            for (k, v) in headers {
                if let Some(v) = v.as_str() {
                    req = req.header(k.as_str(), v);
                }
            }
        }
        let resp = req.send().await.map_err(|e| DeliverError::Http(e.to_string()))?;
        let status = resp.status();
        if !status.is_success() {
            return Err(DeliverError::Status { status: status.as_u16() });
        }
        Ok(status.as_u16())
    }
}
```

`deliver` becomes:

```rust
async fn deliver(&self, event: &Event) -> Result<(), DeliverError> {
    let subs = self.active_subscriptions().await?;
    let matching = subs.iter().filter(|s| {
        topics_match(&s.topics, &event.topic) && project_matches(s.project_id, &event.payload)
    });
    let mut delivered = 0usize;
    for sub in matching {
        delivered += 1;
        // ponytail: all-or-nothing retry per event across subscribers (one flaky
        // consumer re-pings the rest); per-subscription delivery rows if that
        // ever matters — pings are idempotent, so it doesn't today.
        self.deliver_to(sub, event).await?;
    }
    log::debug!(
        "[koji-events] webhook delivered event {} (topic={}) to {} subscription(s)",
        event.id,
        event.topic,
        delivered
    );
    Ok(())
}
```

Also update the module docblock (`//!` header) to describe both modes and project matching. Delete the now-inlined body-serialization from the old `deliver` (the HMAC-over-exact-bytes invariant moved into `deliver_to`).

- [ ] **Step 5: Run the full koji-events unit suite**

Run: `cargo test -p koji-events`
Expected: all pass — the 25 existing webhook tests (signing, topics) plus 4 new `project_matches` tests. Existing tests that construct `Model` literals will now fail to compile until you add the new fields to those literals (`name: "t".into(), project_id: None, mode: "event".into(), method: "GET".into(), headers: None`) — fix them as part of this step.

- [ ] **Step 6: Run koji-events DB integration tests** (dispatcher reads the table)

Run: `set -a; source ./.env.test; set +a; cargo test -p koji-events --test dispatcher_db`
Expected: pass (Task 1's migration already added the columns to the test DB).

- [ ] **Step 7: Commit**

```bash
git add crates/koji-events
git commit -m "feat(events): project-scoped webhook matching + legacy ping delivery mode"
```

---

### Task 3: koji-db — `db/webhook.rs` module (entity + Query + typed enums)

**Files:**
- Create: `crates/koji-db/src/db/webhook.rs`
- Modify: `crates/koji-db/src/db/mod.rs` (add `pub mod webhook;`)
- Modify: `crates/koji-db/src/db/sea_orm_active_enums.rs` (add `WebhookMode`, `WebhookMethod`)
- Modify: `crates/koji-db/src/lib.rs` (re-export `WebhookMode`, `WebhookMethod` at crate root, same as `Category`)
- Modify: `crates/koji-db/src/utils/json.rs` (add `to_webhook` to `JsonToModel`)

**Interfaces:**
- Consumes: Task 1 columns.
- Produces (the `koji_resource!` macro calls these EXACT five on `koji_db::db::webhook::Query` — signatures must match `db/project.rs`'s):
  - `pub async fn paginate(db: &DatabaseConnection, args: AdminReqParsed) -> Result<PaginateResults<Vec<Json>>, DbErr>` — honors `args.q` (name LIKE) and `args.project` (exact `project_id` match).
  - `pub async fn get_one(db: &DatabaseConnection, id: String) -> Result<Model, ModelError>` (id or name).
  - `pub async fn get_one_json(db: &DatabaseConnection, id: String) -> Result<Json, ModelError>`
  - `pub async fn upsert_json_return(db: &DatabaseConnection, id: u32, json: Json) -> Result<Json, ModelError>` (`id == 0` → insert).
  - `pub async fn delete(db: &DatabaseConnection, id: u32) -> Result<DeleteResult, DbErr>`
  - Types: `koji_db::WebhookMode` (`Event`/`Ping`), `koji_db::WebhookMethod` (`Get`/`Post`) — lowercase/uppercase wire strings `"event"`/`"ping"`, `"GET"`/`"POST"`.

- [ ] **Step 1: Study the two files you are mirroring** (5 min, read-only)

Read `crates/koji-db/src/db/project.rs` (Query shape, `#[macros::crud_query]`) and `crates/koji-db/src/db/property.rs` + `sea_orm_active_enums.rs` (ENUM-column pattern). **First check what `#[macros::crud_query]` generates** (`crates/macros/src/lib.rs`, search `crud_query`): if it generates `get_one`/`get_one_json`/`delete` compatible with a `u64` PK + name lookup, use it; if it assumes `u32` ids anywhere, hand-write those three methods instead (code for the hand-written variant is in Step 4 — prefer the attribute if it just works).

- [ ] **Step 2: Add the active enums**

In `crates/koji-db/src/db/sea_orm_active_enums.rs` (mirroring `Category`):

```rust
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "mode")]
pub enum WebhookMode {
    #[sea_orm(string_value = "event")]
    Event,
    #[sea_orm(string_value = "ping")]
    Ping,
}

#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "method")]
pub enum WebhookMethod {
    #[sea_orm(string_value = "GET")]
    Get,
    #[sea_orm(string_value = "POST")]
    Post,
}
```

Re-export from `crates/koji-db/src/lib.rs` next to `Category` (find `pub use` of `Category` and mirror it): `pub use db::sea_orm_active_enums::{WebhookMethod, WebhookMode};`

Note: `Category` additionally has a domain-type + `enum_bridge!` because it predates the wire format. For webhooks the active enums serialize correctly by themselves (`"event"`, `"GET"`); skip the bridge/domain-type unless the macro's `ToSchema` handling (Task 4 Step 2) forces one — YAGNI.

- [ ] **Step 3: Write the failing `to_webhook` test**

In `crates/koji-db/src/utils/json.rs`, find the `JsonToModel` trait and `to_project` (lines ~175-191) for the error style. Add tests (in that file's tests mod, or `db/webhook.rs`'s — put them where `to_project`'s tests live; if it has none, put them in `db/webhook.rs`):

```rust
#[test]
fn to_webhook_minimal_defaults() {
    let m = serde_json::json!({"name": "n", "url": "http://x"}).to_webhook().unwrap();
    assert_eq!(m.name.as_ref(), "n");
    assert_eq!(m.url.as_ref(), "http://x");
    assert_eq!(m.topics.as_ref(), &serde_json::json!([]));
    assert_eq!(m.active.as_ref(), &true);
    assert_eq!(m.mode.as_ref(), &crate::WebhookMode::Event);
    assert_eq!(m.method.as_ref(), &crate::WebhookMethod::Get);
}

#[test]
fn to_webhook_requires_name_and_url() {
    assert!(serde_json::json!({"url": "http://x"}).to_webhook().is_err());
    assert!(serde_json::json!({"name": "n"}).to_webhook().is_err());
}

#[test]
fn to_webhook_full_row() {
    let m = serde_json::json!({
        "name": "reactmap", "url": "http://rm/reload", "secret": "s",
        "topics": ["geofence.updated"], "active": false, "project_id": 7,
        "mode": "ping", "method": "POST", "headers": {"react-map-secret": "v"}
    })
    .to_webhook()
    .unwrap();
    assert_eq!(m.project_id.as_ref(), &Some(7));
    assert_eq!(m.mode.as_ref(), &crate::WebhookMode::Ping);
    assert_eq!(m.method.as_ref(), &crate::WebhookMethod::Post);
}
```

Run: `cargo test -p koji-db to_webhook` → Expected: FAIL (no `to_webhook`).

- [ ] **Step 4: Implement entity + Query + `to_webhook`**

`crates/koji-db/src/db/webhook.rs` (this is the CRUD-side twin of `koji-events`' delivery-side entity — both mirror the DDL in `m20260529_000002` + `m20260706_000001`; the parity canary in Task 4 keeps them honest):

```rust
//! CRUD entity + Query for `webhook_subscription` — the admin-facing twin of
//! `koji_events::entity::webhook_subscription` (which is delivery-facing and
//! stringly-typed). Both mirror the DDL in
//! `crates/migration/src/m20260529_000002_create_event_tables.rs` +
//! `m20260706_000001_webhook_subscription_projects.rs`. The v2_webhooks_db
//! parity test asserts the two entities read the same row identically.

use crate::query_args::AdminReqParsed;
use crate::utils::{json::JsonToModel, parse_order};

use super::sea_orm_active_enums::{WebhookMethod, WebhookMode};
use super::*;
use sea_orm::entity::prelude::*;
use serde_json::json;
use std::str::FromStr;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "webhook_subscription")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: u64,
    pub name: String,
    pub url: String,
    pub secret: Option<String>,
    pub topics: Json,
    pub active: bool,
    pub project_id: Option<u32>,
    pub mode: WebhookMode,
    pub method: WebhookMethod,
    pub headers: Option<Json>,
    pub created_at: DateTime,
    pub updated_at: DateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::project::Entity",
        from = "Column::ProjectId",
        to = "super::project::Column::Id"
    )]
    Project,
}

impl ActiveModelBehavior for ActiveModel {}

pub struct Query;

impl Query {
    pub async fn get_one(db: &DatabaseConnection, id: String) -> Result<Model, ModelError> {
        let record = match id.parse::<u64>() {
            Ok(id) => Entity::find_by_id(id).one(db).await?,
            Err(_) => Entity::find().filter(Column::Name.eq(&id)).one(db).await?,
        };
        record.ok_or(ModelError::Custom("webhook not found".to_string()))
    }

    pub async fn get_one_json(db: &DatabaseConnection, id: String) -> Result<Json, ModelError> {
        Ok(json!(Query::get_one(db, id).await?))
    }

    pub async fn paginate(
        db: &DatabaseConnection,
        args: AdminReqParsed,
    ) -> Result<PaginateResults<Vec<Json>>, DbErr> {
        let mut select = Entity::find()
            .order_by(
                Column::from_str(&args.sort_by).unwrap_or(Column::Name),
                parse_order(&args.order),
            )
            .filter(Column::Name.like(format!("%{}%", args.q).as_str()));
        if let Some(project) = args.project {
            select = select.filter(Column::ProjectId.eq(project));
        }
        let paginator = select.paginate(db, args.per_page);
        let total = paginator.num_items_and_pages().await?;
        let results: Vec<Json> = paginator
            .fetch_page(args.page)
            .await?
            .into_iter()
            .map(|m| json!(m))
            .collect();
        Ok(PaginateResults {
            results,
            total: total.number_of_items,
            has_prev: args.page > 0,
            has_next: args.page + 1 < total.number_of_pages,
        })
    }

    pub async fn upsert(db: &DatabaseConnection, id: u32, json: Json) -> Result<Model, ModelError> {
        let old_model: Option<Model> = Entity::find_by_id(id as u64).one(db).await?;
        let mut new_model = json.to_webhook()?;
        let model = if let Some(old_model) = old_model {
            new_model.id = Set(old_model.id);
            new_model.update(db).await?
        } else {
            new_model.insert(db).await?
        };
        Ok(model)
    }

    pub async fn upsert_json_return(
        db: &DatabaseConnection,
        id: u32,
        json: Json,
    ) -> Result<Json, ModelError> {
        Ok(json!(Query::upsert(db, id, json).await?))
    }

    pub async fn delete(db: &DatabaseConnection, id: u32) -> Result<DeleteResult, DbErr> {
        Entity::delete_by_id(id as u64).exec(db).await
    }
}
```

Notes for the implementer:
- Mirror the exact error/`ModelError` variants `to_project` uses (`json.rs` ~175-191) rather than the `Custom` placeholder above if `crud_query`-generated code uses a dedicated not-found variant — grep how `__not_found_or` in `crates/macros/src/lib.rs` maps `ModelError` to 404 and use the variant that maps to NotFound.
- PATCH semantics: the macro merges by serializing only present fields, but `upsert` above replaces the whole row from JSON. Look at how `project::Query::upsert` handles this — it does the same (`json.to_project()` on the patch value). **Check the macro's `update` handler**: it fetches the existing record first and the patch JSON contains only changed fields, so `to_webhook` on a partial patch would lose fields. Verify how `to_project` + macro PATCH interact for project today (macros/tests/koji_resource.rs has patch assertions) and copy that exact merge behavior. If the macro pre-merges (fetch + overlay) you're fine; if not, `upsert` must overlay patch JSON onto the old model's JSON before `to_webhook` — do what project does.
- `to_webhook` in `utils/json.rs`: required `name`, `url` (else the error variant `to_project` uses for missing fields); optional with defaults: `topics` → `json!([])`, `active` → `true`, `mode` → `WebhookMode::Event`, `method` → `WebhookMethod::Get`; passthrough optional: `secret`, `project_id`, `headers`. Parse mode/method from their wire strings via `serde_json::from_value` on the enum. Reject non-object `headers` with an error (validation at the trust boundary), set timestamps the way `to_project` does.

- [ ] **Step 5: Run tests**

Run: `cargo test -p koji-db to_webhook` → Expected: PASS.
Run: `cargo build -p koji-db` → Expected: clean (warnings ok only if pre-existing).

- [ ] **Step 6: Commit**

```bash
git add crates/koji-db
git commit -m "feat(db): webhook_subscription CRUD module with typed mode/method enums"
```

---

### Task 4: koji-service — `/api/v2/webhooks` resource (6th `koji_resource!`)

**Files:**
- Modify: `crates/koji-service/src/public/v2/resources.rs` (new invocation)
- Modify: `crates/koji-service/src/lib.rs` (~line 600: mount scope)
- Modify: `crates/koji-service/src/internal/mod.rs` (~line 51: mount under `/internal`)
- Modify: `crates/koji-service/src/utils/openapi.rs` (paths + schemas + tag)
- Create: `crates/koji-service/tests/v2_webhooks_db.rs`

**Interfaces:**
- Consumes: Task 3's `koji_db::db::webhook::Query` + `koji_db::{WebhookMode, WebhookMethod}`.
- Produces: `GET/POST /api/v2/webhooks`, `GET/PATCH/DELETE /api/v2/webhooks/{id}` (+ `/internal/webhooks/**` mirror); list honors `?project=`. Realtime hub topic `"webhook"`. `CreateWebhook`/`PatchWebhook` DTOs.

- [ ] **Step 1: Write the failing test** — `crates/koji-service/tests/v2_webhooks_db.rs`

Copy the header/helpers pattern from `crates/koji-service/tests/v2_db.rs` (`test_db()`, `build_test_koji_db()`, `body_json()`, `unique_name()`, `serial_guard()` — import or duplicate exactly as other test files do; check whether helpers live in a shared module or are per-file):

```rust
#[actix_web::test]
async fn webhooks_full_crud_cycle_and_project_filter() {
    let Some(db) = test_db().await else { return };
    let _serial = serial_guard();
    let name = unique_name("hook");
    let koji_db = build_test_koji_db(db.clone()).await;
    let jobs = Arc::new(JobQueue::new(db.clone(), "test-worker"));
    let app = test::init_service(koji_service::test_db_app(koji_db, jobs)).await;

    // Create a project to scope against.
    let project_name = unique_name("proj");
    let req = test::TestRequest::post()
        .uri("/api/v2/projects")
        .set_json(serde_json::json!({"name": project_name, "golbat": false}))
        .to_request();
    // NOTE: after Task 9 lands, drop the `golbat` field from this body.
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    let project_id = body_json(resp).await["data"]["id"].as_u64().unwrap();

    // ── POST /api/v2/webhooks → 201 + Location ─────────────────────────
    let req = test::TestRequest::post()
        .uri("/api/v2/webhooks")
        .set_json(serde_json::json!({
            "name": name, "url": "http://127.0.0.1:1/reload",
            "mode": "ping", "method": "POST",
            "project_id": project_id,
            "headers": {"x-golbat-secret": "abc"}
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201, "create webhook must return 201");
    let created = body_json(resp).await;
    let id = created["data"]["id"].as_u64().unwrap();

    // ── GET one (by id) ────────────────────────────────────────────────
    let req = test::TestRequest::get()
        .uri(&format!("/api/v2/webhooks/{id}"))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let got = body_json(resp).await;
    assert_eq!(got["data"]["mode"], "ping");
    assert_eq!(got["data"]["method"], "POST");
    assert_eq!(got["data"]["headers"]["x-golbat-secret"], "abc");
    assert_eq!(got["data"]["project_id"], project_id);
    assert_eq!(got["data"]["topics"], serde_json::json!([]));

    // ── LIST with ?project= filter ─────────────────────────────────────
    let req = test::TestRequest::get()
        .uri(&format!("/api/v2/webhooks?project={project_id}"))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let list = body_json(resp).await;
    assert!(list["data"].as_array().unwrap().iter().any(|w| w["id"].as_u64() == Some(id)));
    let req = test::TestRequest::get()
        .uri("/api/v2/webhooks?project=999999999")
        .to_request();
    let resp = test::call_service(&app, req).await;
    let list = body_json(resp).await;
    assert!(list["data"].as_array().unwrap().is_empty());

    // ── PATCH ──────────────────────────────────────────────────────────
    let req = test::TestRequest::patch()
        .uri(&format!("/api/v2/webhooks/{id}"))
        .set_json(serde_json::json!({"active": false}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let patched = body_json(resp).await;
    assert_eq!(patched["data"]["active"], false);
    assert_eq!(patched["data"]["url"], "http://127.0.0.1:1/reload", "PATCH must not clobber other fields");

    // ── Parity canary: koji-events entity reads the same row ──────────
    let ev = koji_events::entity::webhook_subscription::Entity::find_by_id(id)
        .one(&db)
        .await
        .unwrap()
        .expect("koji-events entity must read the row koji-db wrote");
    assert_eq!(ev.mode, "ping");
    assert_eq!(ev.method, "POST");
    assert_eq!(ev.project_id, Some(project_id as u32));

    // ── DELETE → 204; project cascade cleanup ──────────────────────────
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v2/webhooks/{id}"))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 204);
    // Cleanup the project (cascade already exercised implicitly elsewhere).
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v2/projects/{project_id}"))
        .to_request();
    test::call_service(&app, req).await;
}
```

Add a second test: `webhook_cascade_dies_with_project` — create project + scoped webhook, DELETE the project, assert `GET /api/v2/webhooks/{id}` → 404.

- [ ] **Step 2: Run tests to verify failure**

Run: `set -a; source ./.env.test; set +a; cargo test -p koji-service --test v2_webhooks_db`
Expected: FAIL — 404 on `/api/v2/webhooks` (route absent). (Compile error first if imports miss — `koji_events` must be in koji-service's deps; it is, from boot wiring.)

- [ ] **Step 3: Add the macro invocation + mounts + openapi**

In `resources.rs` (after tile_server invocation):

```rust
koji_resource! {
    module: webhook,
    seg: "webhooks",
    topic: "webhook",
    create: {
        name: String,
        url: String,
        secret: Option<String>,
        topics: Option<serde_json::Value>,
        active: Option<bool>,
        project_id: Option<u32>,
        mode: Option<koji_db::WebhookMode>,
        method: Option<koji_db::WebhookMethod>,
        headers: Option<serde_json::Value>,
    }
}
```

(If the macro's non-primitive `ToSchema` hinting rejects `koji_db::WebhookMode`, check how `koji_db::Category` is declared for `property` — same mechanism, `#[schema(value_type = String)]` is added automatically per macros/src/lib.rs ~661-695.)

Mounts:
- `crates/koji-service/src/lib.rs` (~line 600, next to the other resources): `.service(public::v2::resources::webhook::scope())`
- `crates/koji-service/src/internal/mod.rs` (~line 51): same line in the internal scope.

OpenAPI (`utils/openapi.rs`): add the 5 paths (`crate::public::v2::resources::webhook::{list, create, get_one, update, remove}`), 2 schemas (`CreateWebhook`, `PatchWebhook`), and a tag `(name = "webhooks", description = "Webhook subscription CRUD")` — mirror the projects entries exactly.

- [ ] **Step 4: Run tests to verify pass**

Run: `set -a; source ./.env.test; set +a; cargo test -p koji-service --test v2_webhooks_db`
Expected: PASS (both tests). If PATCH clobbers fields, revisit Task 3 Step 4's merge note — fix in koji-db, not here.

- [ ] **Step 5: Commit**

```bash
git add crates/koji-service
git commit -m "feat(api): /api/v2/webhooks CRUD resource with ?project= filter"
```

---

### Task 5: `POST /api/v2/webhooks/{id}/test` — manual fire

**Files:**
- Create: `crates/koji-service/src/public/v2/webhooks_test.rs`
- Modify: `crates/koji-service/src/public/v2/mod.rs` (add module)
- Modify: `crates/koji-service/src/lib.rs` + `crates/koji-service/src/internal/mod.rs` (mount BEFORE the webhook scope)
- Modify: `crates/koji-service/src/utils/openapi.rs`
- Test: extend `crates/koji-service/tests/v2_webhooks_db.rs`

**Interfaces:**
- Consumes: Task 2's `WebhookSubscriber::deliver_to`, `koji_events::entity::webhook_subscription`, `koji_events::types::{Event, EventId}`.
- Produces: `POST /api/v2/webhooks/{id}/test` → `200 {"status":"ok","data":{"delivered":bool,"upstream_status":u16|null,"error":string|null}}`, `404` unknown id. Fires synchronously, does NOT touch the outbox.

- [ ] **Step 1: Write the failing tests** (append to `v2_webhooks_db.rs`)

```rust
#[actix_web::test]
async fn webhook_test_endpoint_404_on_missing() {
    let Some(db) = test_db().await else { return };
    let _serial = serial_guard();
    let koji_db = build_test_koji_db(db.clone()).await;
    let jobs = Arc::new(JobQueue::new(db.clone(), "test-worker"));
    let app = test::init_service(koji_service::test_db_app(koji_db, jobs)).await;
    let req = test::TestRequest::post()
        .uri("/api/v2/webhooks/999999999/test")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 404);
}

#[actix_web::test]
async fn webhook_test_endpoint_fires_ping_at_live_listener() {
    let Some(db) = test_db().await else { return };
    let _serial = serial_guard();

    // Real listener on an OS-assigned port; records method + headers.
    use std::sync::{Arc as StdArc, Mutex};
    #[derive(Clone, Default)]
    struct Seen(StdArc<Mutex<Vec<(String, Option<String>, Option<String>)>>>);
    let seen = Seen::default();
    let seen_c = seen.clone();
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let server = actix_web::HttpServer::new(move || {
        let seen = seen_c.clone();
        actix_web::App::new().default_service(actix_web::web::to(
            move |req: actix_web::HttpRequest| {
                let seen = seen.clone();
                async move {
                    let hdr = |n: &str| {
                        req.headers().get(n).and_then(|v| v.to_str().ok()).map(String::from)
                    };
                    seen.0.lock().unwrap().push((
                        req.method().to_string(),
                        hdr("x-golbat-secret"),
                        hdr("X-Koji-Event-Id"),
                    ));
                    actix_web::HttpResponse::Ok().finish()
                }
            },
        ))
    })
    .listen(listener)
    .unwrap()
    .workers(1)
    .run();
    let handle = server.handle();
    tokio::spawn(server);

    let koji_db = build_test_koji_db(db.clone()).await;
    let jobs = Arc::new(JobQueue::new(db.clone(), "test-worker"));
    let app = test::init_service(koji_service::test_db_app(koji_db, jobs)).await;
    let name = unique_name("hook");
    let req = test::TestRequest::post()
        .uri("/api/v2/webhooks")
        .set_json(serde_json::json!({
            "name": name, "url": format!("http://127.0.0.1:{port}/reload"),
            "mode": "ping", "method": "POST",
            "headers": {"x-golbat-secret": "abc"}
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let id = body_json(resp).await["data"]["id"].as_u64().unwrap();

    let req = test::TestRequest::post()
        .uri(&format!("/api/v2/webhooks/{id}/test"))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let status = resp.status().as_u16();
    let body = body_json(resp).await;

    // Cleanup before asserting.
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v2/webhooks/{id}"))
        .to_request();
    test::call_service(&app, req).await;
    handle.stop(true).await;

    assert_eq!(status, 200);
    assert_eq!(body["data"]["delivered"], true);
    assert_eq!(body["data"]["upstream_status"], 200);
    let seen = seen.0.lock().unwrap();
    assert_eq!(seen.len(), 1, "exactly one ping");
    assert_eq!(seen[0].0, "POST", "honors method");
    assert_eq!(seen[0].1.as_deref(), Some("abc"), "custom header sent");
    assert!(seen[0].2.is_some(), "X-Koji-Event-Id sent");
}
```

Run: `set -a; source ./.env.test; set +a; cargo test -p koji-service --test v2_webhooks_db webhook_test_endpoint`
Expected: FAIL — 404 both (route absent) / second test fails on `delivered`.

- [ ] **Step 2: Implement the handler** — `crates/koji-service/src/public/v2/webhooks_test.rs`

```rust
//! `POST /api/v2/webhooks/{id}/test` — synchronously fire one subscription with
//! a sample event and report the upstream result inline. Replaces v1's
//! "sync-to-test" workflow; deliberately bypasses the outbox (no retries — the
//! admin is watching the response).

use actix_web::{web, HttpResponse};
use koji_events::entity::webhook_subscription;
use koji_events::types::{Event, EventId};
use koji_events::WebhookSubscriber;
use sea_orm::EntityTrait;

use crate::utils::api_response::ApiResponse;
use crate::utils::error::ServiceError;

#[utoipa::path(
    post,
    path = "/api/v2/webhooks/{id}/test",
    tag = "webhooks",
    params(("id" = u64, Path, description = "Webhook subscription id")),
    responses(
        (status = 200, description = "Delivery attempted; result in body", body = Object),
        (status = 404, description = "No such webhook", body = crate::utils::api_response::ApiError),
    ),
)]
pub(crate) async fn test_fire(
    db: web::Data<koji_db::KojiDb>,
    path: web::Path<u64>,
) -> Result<HttpResponse, ServiceError> {
    let id = path.into_inner();
    let sub = webhook_subscription::Entity::find_by_id(id)
        .one(&db.koji)
        .await
        .map_err(ServiceError::internal)?
        .ok_or(ServiceError::NotFound {
            field: "webhook",
            message: "does not exist".to_string(),
        })?;
    let event = Event {
        id: EventId::new().as_string(),
        topic: "webhook.test".to_string(),
        payload: serde_json::json!({
            "test": true,
            "webhookId": sub.id,
            "projectId": sub.project_id,
        }),
    };
    let subscriber = WebhookSubscriber::with_default_client(db.koji.clone());
    let data = match subscriber.deliver_to(&sub, &event).await {
        Ok(status) => serde_json::json!({
            "delivered": true, "upstream_status": status, "error": null
        }),
        Err(e) => serde_json::json!({
            "delivered": false, "upstream_status": null, "error": e.to_string()
        }),
    };
    Ok(ApiResponse::success(data))
}
```

(Adjust `ServiceError` variant construction and `ApiResponse::success` to the exact shapes used in `geofences.rs` — same imports, same envelope. Check `koji_events::WebhookSubscriber` is re-exported at crate root — boot wiring in `lib.rs` already imports it, follow that path.)

Register module in `public/v2/mod.rs`: `pub(crate) mod webhooks_test;`

Mount in `lib.rs` BEFORE the webhook scope (actix must match `/webhooks/{id}/test` before the scope's `/{id}`; verify order by test — if the scope still wins, mount the test route inside a wrapper: replace the plain mount with `web::scope("/webhooks").service(web::resource("/{id}/test").route(web::post().to(...)))` — no: simplest correct form is to register the more specific route first at the same level):

```rust
.service(
    web::resource("/webhooks/{id}/test")
        .route(web::post().to(public::v2::webhooks_test::test_fire)),
)
.service(public::v2::resources::webhook::scope())
```

Same two lines in `internal/mod.rs`. Add the path fn to openapi.rs `paths(...)`.

- [ ] **Step 3: Run tests to verify pass**

Run: `set -a; source ./.env.test; set +a; cargo test -p koji-service --test v2_webhooks_db`
Expected: all 4 tests PASS.

- [ ] **Step 4: Commit**

```bash
git add crates/koji-service
git commit -m "feat(api): POST /api/v2/webhooks/{id}/test — synchronous single-subscription fire"
```

---

### Task 6: `koji_resource!` outbox flag → `project.updated` / `project.deleted`

**Files:**
- Modify: `crates/macros/src/lib.rs` (optional `outbox: true` invocation param)
- Modify: `crates/koji-service/src/public/v2/resources.rs` (project invocation gains `outbox: true`)
- Modify: `crates/macros/tests/koji_resource.rs` (only if fixtures break — see note)
- Test: extend `crates/koji-service/tests/v2_webhooks_db.rs` (or new `v2_project_events_db.rs`)

**Interfaces:**
- Consumes: `koji_events::EventDispatcher::publish(db: &DatabaseConnection, topic: &str, payload: &impl Serialize) -> Result<EventId, PublishError>` (associated fn, no instance).
- Produces: with `outbox: true`, the generated `update` handler publishes topic `"{topic}.updated"` payload `{"projectId": id, "name": <record name>}` and `remove` publishes `"{topic}.deleted"` with the same shape. Other invocations are unaffected (no flag = no outbox code generated).

- [ ] **Step 1: Write the failing test**

```rust
#[actix_web::test]
async fn project_patch_and_delete_emit_outbox_events() {
    let Some(db) = test_db().await else { return };
    let _serial = serial_guard();
    let koji_db = build_test_koji_db(db.clone()).await;
    let jobs = Arc::new(JobQueue::new(db.clone(), "test-worker"));
    let app = test::init_service(koji_service::test_db_app(koji_db, jobs)).await;

    let name = unique_name("proj");
    let req = test::TestRequest::post()
        .uri("/api/v2/projects")
        .set_json(serde_json::json!({"name": name, "golbat": false}))
        .to_request();
    let project_id = body_json(test::call_service(&app, req).await).await["data"]["id"]
        .as_u64()
        .unwrap();

    let new_name = unique_name("proj2");
    let req = test::TestRequest::patch()
        .uri(&format!("/api/v2/projects/{project_id}"))
        .set_json(serde_json::json!({"name": new_name}))
        .to_request();
    assert_eq!(test::call_service(&app, req).await.status(), 200);

    let req = test::TestRequest::delete()
        .uri(&format!("/api/v2/projects/{project_id}"))
        .to_request();
    assert_eq!(test::call_service(&app, req).await.status(), 204);

    // Outbox rows exist for both topics with the project id in payload.
    use sea_orm::{ConnectionTrait, Statement};
    let fetch = |topic: &'static str| {
        let db = db.clone();
        async move {
            db.query_all(Statement::from_sql_and_values(
                sea_orm::DbBackend::MySql,
                "SELECT CAST(payload AS CHAR) AS p FROM event_outbox WHERE topic = ? \
                 AND JSON_EXTRACT(payload, '$.projectId') = ?",
                [topic.into(), project_id.into()],
            ))
            .await
            .unwrap()
        }
    };
    let updated = fetch("project.updated").await;
    let deleted = fetch("project.deleted").await;
    // Cleanup outbox rows before asserting (panic-safe).
    db.execute(Statement::from_sql_and_values(
        sea_orm::DbBackend::MySql,
        "DELETE FROM event_outbox WHERE JSON_EXTRACT(payload, '$.projectId') = ?",
        [project_id.into()],
    ))
    .await
    .unwrap();
    assert!(!updated.is_empty(), "PATCH must emit project.updated");
    assert!(!deleted.is_empty(), "DELETE must emit project.deleted");
}
```

Run: `set -a; source ./.env.test; set +a; cargo test -p koji-service --test v2_webhooks_db project_patch_and_delete`
Expected: FAIL — no outbox rows.

- [ ] **Step 2: Extend the macro**

In `crates/macros/src/lib.rs`, `koji_resource!`'s parser: accept an optional `outbox: true,` key after `topic:` (parse an optional `outbox` ident + bool literal; default false). When true, generate — in `update`, after the `hub.publish` loop (`record` is in scope):

```rust
if let Err(e) = koji_events::EventDispatcher::publish(
    &db.koji,
    concat!(#topic, ".updated"),
    &serde_json::json!({
        "projectId": id,
        "name": record.get("name").cloned().unwrap_or(serde_json::Value::Null),
    }),
)
.await
{
    log::warn!(concat!("[", #seg, "] outbox publish failed: {}"), e);
}
```

And in `remove` — the handler needs the record's name for the payload, and it currently doesn't fetch before deleting. When (and only when) `outbox: true`, generate a pre-fetch at the top of `remove`:

```rust
let __name = koji_db::db::#module::Query::get_one(&db.koji, id.to_string())
    .await
    .ok()
    .map(|m| serde_json::json!(m.name))
    .unwrap_or(serde_json::Value::Null);
```

then after the rows_affected check + hub publish:

```rust
if let Err(e) = koji_events::EventDispatcher::publish(
    &db.koji,
    concat!(#topic, ".deleted"),
    &serde_json::json!({ "projectId": id, "name": __name }),
)
.await
{
    log::warn!(concat!("[", #seg, "] outbox publish failed: {}"), e);
}
```

Payload key is `projectId` because the only `outbox: true` consumer is project (webhook project-matching reads `projectId`). Do NOT genericize the key name (`YAGNI`); if a second resource ever opts in, revisit.

Set `outbox: true` on the **project** invocation in `resources.rs` only.

Note on `crates/macros/tests/koji_resource.rs`: these tests expand the macro inside the macros crate's test build, which does not depend on `koji_events`. If a fixture there uses `outbox: true` it won't compile — don't add one; the expansion for `outbox: false`/absent must stay byte-identical to before (assert existing fixtures still pass untouched). The `outbox: true` path is covered by the koji-service integration test.

- [ ] **Step 3: Run tests**

Run (one batch, parallel):
- `cargo test -p macros` → existing expansion fixtures still PASS.
- `set -a; source ./.env.test; set +a; cargo test -p koji-service --test v2_webhooks_db` → all PASS including the new one.

- [ ] **Step 4: Commit**

```bash
git add crates/macros crates/koji-service
git commit -m "feat(events): project PATCH/DELETE emit project.updated/deleted via koji_resource outbox flag"
```

---

### Task 7: `geofence.updated` + `route.updated` emission

**Files:**
- Modify: `crates/koji-db/src/db/geofence_project.rs` (add `project_ids_for_geofence`)
- Modify: `crates/koji-service/src/public/v2/geofences.rs` (create + update handlers)
- Modify: `crates/koji-service/src/public/v2/routes.rs` (create + update handlers)
- Test: extend `crates/koji-service/tests/v2_webhooks_db.rs`

**Interfaces:**
- Consumes: `EventDispatcher::publish`, geofence/route handler internals (`record` JSON after upsert).
- Produces:
  - `pub async fn geofence_project::Query::project_ids_for_geofence(db: &DatabaseConnection, geofence_id: u32) -> Result<Vec<u32>, DbErr>`
  - Topic `geofence.updated`, payload `{"geofenceId": id, "name": <name>, "projectIds": [u32]}` — from geofence POST and PATCH.
  - Topic `route.updated`, payload `{"routeId": id, "geofenceId": <parent>, "name": <name>, "projectIds": [u32]}` — from route POST and PATCH.
  - Shared helper in koji-service: `pub(crate) async fn emit_event(db: &sea_orm::DatabaseConnection, topic: &str, payload: serde_json::Value)` (log-warn wrapper) — put it in a new `crates/koji-service/src/utils/outbox.rs`, used by Tasks 7–8.

- [ ] **Step 1: Write the failing test**

```rust
#[actix_web::test]
async fn geofence_update_emits_event_with_project_ids() {
    let Some(db) = test_db().await else { return };
    let _serial = serial_guard();
    let koji_db = build_test_koji_db(db.clone()).await;
    let jobs = Arc::new(JobQueue::new(db.clone(), "test-worker"));
    let app = test::init_service(koji_service::test_db_app(koji_db, jobs)).await;

    // project + geofence linked to it (geofence create accepts "projects").
    let pname = unique_name("proj");
    let req = test::TestRequest::post()
        .uri("/api/v2/projects")
        .set_json(serde_json::json!({"name": pname, "golbat": false}))
        .to_request();
    let project_id = body_json(test::call_service(&app, req).await).await["data"]["id"].as_u64().unwrap();

    let gname = unique_name("fence");
    let req = test::TestRequest::post()
        .uri("/api/v2/geofences")
        .set_json(serde_json::json!({
            "name": gname, "mode": "pokemon", "geometry": triangle_geometry(),
            "projects": [project_id]
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    let geofence_id = body_json(resp).await["data"]["id"].as_u64().unwrap();

    // The create itself must have emitted geofence.updated w/ projectIds.
    use sea_orm::{ConnectionTrait, Statement};
    let rows = db
        .query_all(Statement::from_sql_and_values(
            sea_orm::DbBackend::MySql,
            "SELECT CAST(payload AS CHAR) AS p FROM event_outbox WHERE topic = 'geofence.updated' \
             AND JSON_EXTRACT(payload, '$.geofenceId') = ?",
            [geofence_id.into()],
        ))
        .await
        .unwrap();
    // Cleanup (fence cascade-cleans geofence_project; outbox rows by hand).
    cleanup_geofence(&db, geofence_id).await;
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v2/projects/{project_id}"))
        .to_request();
    test::call_service(&app, req).await;
    db.execute(Statement::from_sql_and_values(
        sea_orm::DbBackend::MySql,
        "DELETE FROM event_outbox WHERE JSON_EXTRACT(payload, '$.geofenceId') = ?",
        [geofence_id.into()],
    ))
    .await
    .unwrap();

    assert!(!rows.is_empty(), "geofence create must emit geofence.updated");
    let payload: serde_json::Value = serde_json::from_str(
        rows[0].try_get::<String>("", "p").unwrap().as_str(),
    )
    .unwrap();
    assert!(
        payload["projectIds"].as_array().unwrap().iter().any(|v| v.as_u64() == Some(project_id)),
        "payload must carry linked projectIds: {payload}"
    );
}
```

(If geofence create's DTO field for project links is not `"projects"`, check `CreateGeofence` in `geofences.rs` + `geofence/writes.rs:77-89` `upsert_related_projects` for the actual key and adjust the test body.)

Add the route twin: create route under that geofence via `POST /api/v2/routes` (copy an existing route-create body from `v2_db.rs`), assert `route.updated` outbox row with `projectIds` containing the project.

Run: `set -a; source ./.env.test; set +a; cargo test -p koji-service --test v2_webhooks_db geofence_update_emits`
Expected: FAIL — no outbox rows.

- [ ] **Step 2: Implement**

koji-db helper (`geofence_project.rs`, next to the upsert helpers):

```rust
/// The project ids a geofence is currently linked to (for event payloads).
pub async fn project_ids_for_geofence(
    db: &DatabaseConnection,
    geofence_id: u32,
) -> Result<Vec<u32>, DbErr> {
    Ok(Entity::find()
        .filter(Column::GeofenceId.eq(geofence_id))
        .all(db)
        .await?
        .into_iter()
        .map(|m| m.project_id)
        .collect())
}
```

Service helper `crates/koji-service/src/utils/outbox.rs` (+ `pub(crate) mod outbox;` in `utils/mod.rs`):

```rust
//! Outbox emission helper for request handlers: publish-or-warn, never fail
//! the request (a missed webhook ping is not data loss).

pub(crate) async fn emit_event(
    db: &sea_orm::DatabaseConnection,
    topic: &str,
    payload: serde_json::Value,
) {
    if let Err(e) = koji_events::EventDispatcher::publish(db, topic, &payload).await {
        log::warn!("[outbox] publish {topic} failed: {e}");
    }
}
```

In `geofences.rs` `create` (after upsert + hub publish, `record` in scope) and `update` (same spot):

```rust
let project_ids = koji_db::db::geofence_project::Query::project_ids_for_geofence(&conn.koji, id as u32)
    .await
    .unwrap_or_default();
crate::utils::outbox::emit_event(
    &conn.koji,
    "geofence.updated",
    serde_json::json!({
        "geofenceId": id,
        "name": record.get("name").cloned().unwrap_or(serde_json::Value::Null),
        "projectIds": project_ids,
    }),
)
.await;
```

(Adapt variable names to each handler — `conn` vs `db`, where `id` comes from `record["id"]` on create vs the path on update. The geofence record is a GeoJSON feature — `name` may live at `record["properties"]["name"]`; check the actual record shape from the handler's `record` value and use the right path.)

In `routes.rs` `create`/`update` — route records carry `geofence_id`; resolve projects through the parent:

```rust
let geofence_id = record
    .get("geofence_id")
    .and_then(serde_json::Value::as_u64)
    .unwrap_or_default() as u32;
let project_ids = koji_db::db::geofence_project::Query::project_ids_for_geofence(&conn.koji, geofence_id)
    .await
    .unwrap_or_default();
crate::utils::outbox::emit_event(
    &conn.koji,
    "route.updated",
    serde_json::json!({
        "routeId": id,
        "geofenceId": geofence_id,
        "name": record.get("name").cloned().unwrap_or(serde_json::Value::Null),
        "projectIds": project_ids,
    }),
)
.await;
```

- [ ] **Step 3: Run tests**

Run: `set -a; source ./.env.test; set +a; cargo test -p koji-service --test v2_webhooks_db`
Expected: all PASS.

- [ ] **Step 4: Commit**

```bash
git add crates/koji-db crates/koji-service
git commit -m "feat(events): geofence.updated + route.updated emission with projectIds payload"
```

---

### Task 8: `project.geofences_changed` emission (membership diffs, bulk-aware)

**Files:**
- Modify: `crates/koji-db/src/db/geofence_project.rs` (add `geofence_ids_for_project`)
- Modify: `crates/koji-service/src/public/v2/geofences.rs` (create/update: per-project membership diff)
- Modify: project PATCH path — via `resources.rs`/macro? NO: project membership changes flow through `koji_db::db::project::Query::upsert → upsert_related_geofences` when the PATCH body contains `"geofences"`. The macro handler is sealed, so emit from a small wrapper: see Step 2.
- Modify: the import handler (find it: `grep -rn "import" crates/koji-service/src/internal/` — the atomic `/internal/import` endpoint from the import-wizard feature)
- Test: extend `crates/koji-service/tests/v2_webhooks_db.rs`

**Interfaces:**
- Consumes: Task 7's helpers (`project_ids_for_geofence`, `emit_event`).
- Produces:
  - `pub async fn geofence_project::Query::geofence_ids_for_project(db, project_id: u32) -> Result<Vec<u32>, DbErr>` (mirror of Task 7's helper).
  - Topic `project.geofences_changed`, payload `{"projectId": u32, "addedIds": [u32], "removedIds": [u32]}`.
  - **One event per affected project per HTTP operation** — the bulk import of N fences emits ≤ (number of affected projects) events, never N.

- [ ] **Step 1: Write the failing tests**

Test A — geofence PATCH that changes its project links emits one event per affected project:

```rust
#[actix_web::test]
async fn geofence_membership_change_emits_geofences_changed() {
    let Some(db) = test_db().await else { return };
    let _serial = serial_guard();
    let koji_db = build_test_koji_db(db.clone()).await;
    let jobs = Arc::new(JobQueue::new(db.clone(), "test-worker"));
    let app = test::init_service(koji_service::test_db_app(koji_db, jobs)).await;

    // Two projects; fence starts in A, moves to B.
    let mut ids = vec![];
    for _ in 0..2 {
        let req = test::TestRequest::post()
            .uri("/api/v2/projects")
            .set_json(serde_json::json!({"name": unique_name("proj"), "golbat": false}))
            .to_request();
        ids.push(body_json(test::call_service(&app, req).await).await["data"]["id"].as_u64().unwrap());
    }
    let (a, b) = (ids[0], ids[1]);
    let req = test::TestRequest::post()
        .uri("/api/v2/geofences")
        .set_json(serde_json::json!({
            "name": unique_name("fence"), "mode": "pokemon",
            "geometry": triangle_geometry(), "projects": [a]
        }))
        .to_request();
    let fence = body_json(test::call_service(&app, req).await).await["data"]["id"].as_u64().unwrap();

    let req = test::TestRequest::patch()
        .uri(&format!("/api/v2/geofences/{fence}"))
        .set_json(serde_json::json!({"projects": [b]}))
        .to_request();
    assert_eq!(test::call_service(&app, req).await.status(), 200);

    use sea_orm::{ConnectionTrait, Statement};
    let rows = db
        .query_all(Statement::from_sql_and_values(
            sea_orm::DbBackend::MySql,
            "SELECT CAST(payload AS CHAR) AS p FROM event_outbox \
             WHERE topic = 'project.geofences_changed' \
             AND JSON_EXTRACT(payload, '$.projectId') IN (?, ?)",
            [a.into(), b.into()],
        ))
        .await
        .unwrap();
    // cleanup fence/projects/outbox as in prior tests ...
    let payloads: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| serde_json::from_str(r.try_get::<String>("", "p").unwrap().as_str()).unwrap())
        .collect();
    // A lost the fence, B gained it (from the PATCH; the create also emitted one for A).
    assert!(payloads.iter().any(|p| p["projectId"].as_u64() == Some(a)
        && p["removedIds"].as_array().unwrap().iter().any(|v| v.as_u64() == Some(fence))));
    assert!(payloads.iter().any(|p| p["projectId"].as_u64() == Some(b)
        && p["addedIds"].as_array().unwrap().iter().any(|v| v.as_u64() == Some(fence))));
}
```

Test B — bulk import emits ONE event per project (adapt to the import endpoint's actual request shape — read the import handler + its existing test first; the assertion that matters: import 3 fences all linked to one project → exactly 1 `project.geofences_changed` row for that project, `addedIds` len 3).

Run: expected FAIL.

- [ ] **Step 2: Implement**

koji-db mirror helper:

```rust
/// The geofence ids linked to a project (for membership-diff payloads).
pub async fn geofence_ids_for_project(
    db: &DatabaseConnection,
    project_id: u32,
) -> Result<Vec<u32>, DbErr> {
    Ok(Entity::find()
        .filter(Column::ProjectId.eq(project_id))
        .all(db)
        .await?
        .into_iter()
        .map(|m| m.geofence_id)
        .collect())
}
```

Diff-and-emit helper in `utils/outbox.rs`:

```rust
/// Emit `project.geofences_changed` for every project whose membership diff
/// (before vs after) is non-empty. `before`/`after` map project_id → geofence
/// ids linked through it. One event per project per operation (spec §4).
pub(crate) async fn emit_membership_diff(
    db: &sea_orm::DatabaseConnection,
    before: &std::collections::HashMap<u32, std::collections::BTreeSet<u32>>,
    after: &std::collections::HashMap<u32, std::collections::BTreeSet<u32>>,
) {
    let projects: std::collections::BTreeSet<u32> =
        before.keys().chain(after.keys()).copied().collect();
    for pid in projects {
        let empty = std::collections::BTreeSet::new();
        let b = before.get(&pid).unwrap_or(&empty);
        let a = after.get(&pid).unwrap_or(&empty);
        let added: Vec<u32> = a.difference(b).copied().collect();
        let removed: Vec<u32> = b.difference(a).copied().collect();
        if added.is_empty() && removed.is_empty() {
            continue;
        }
        emit_event(
            db,
            "project.geofences_changed",
            serde_json::json!({ "projectId": pid, "addedIds": added, "removedIds": removed }),
        )
        .await;
    }
}
```

Call sites (each builds `before` pre-mutation and `after` post-mutation):

1. **Geofence create/update** (`geofences.rs`): only when the body carries the project-links key (`"projects"`). Before the upsert: `let before_pids = project_ids_for_geofence(...)` (empty on create); after: `let after_pids = ...`. Build the two maps with this single fence: `{pid → {fence_id}}` for each pid in the respective lists, then `emit_membership_diff`. This composes with Task 7's `geofence.updated` emission (both fire; different topics, different consumers).
2. **Project PATCH with `"geofences"`**: the macro handler is sealed but the membership write happens inside `project::Query::upsert`. Don't touch the macro again — instead snapshot in the same place the macro allows: NO hook exists, so move this emission into koji-db? NO (koji-db must not write outbox). Resolution: the ONLY v2 path that sends `"geofences"` on a project PATCH is the admin UI's bulk-assign (E9.1, not yet built). ponytail: skip project-PATCH-side emission, document it in the code with `// ponytail: project PATCH "geofences" bulk-assign emits no membership events yet — wire when E9.1 builds the reverse-assign UI (snapshot before/after around Query::upsert in a thin unsealed wrapper or macro hook)`. Put that comment on `upsert_related_geofences` in `project.rs`. Test B/assertions do NOT cover this path.
3. **Import handler** (`internal/…`): find where it loops feature upserts inside the transaction. Snapshot each affected project's `geofence_ids_for_project` BEFORE the tx (or accumulate added ids in the loop — the import creates fences, so `before` has no rows for them; accumulating `{pid → added fence ids}` in the loop is simpler and avoids pre-tx snapshots), then after the tx commits, call `emit_membership_diff(&conn.koji, &HashMap::new(), &accumulated)`. Emitting AFTER commit is required — an event for a rolled-back import is a lie. If fences in the import can also *replace* existing project links, fall back to the before/after snapshot form.

- [ ] **Step 3: Run tests**

Run: `set -a; source ./.env.test; set +a; cargo test -p koji-service --test v2_webhooks_db`
Expected: all PASS, including bulk-import single-event assertion.

- [ ] **Step 4: Commit**

```bash
git add crates/koji-db crates/koji-service
git commit -m "feat(events): project.geofences_changed membership diffs, bulk-aware import emission"
```

---

### Task 9: Migration B + drop v1 push fields from all project code

**Files:**
- Create: `crates/migration/src/m20260706_000002_drop_project_push_columns.rs`
- Modify: `crates/migration/src/lib.rs` (register)
- Modify: `crates/koji-db/src/db/project.rs` (Model fields, `paginate` json!, delete `get_golbat_project`)
- Modify: `crates/koji-db/src/utils/json.rs` (`to_project`: drop api_endpoint/api_key/golbat parsing)
- Modify: `crates/koji-service/src/public/v2/resources.rs` (project invocation: drop the 3 fields)
- Modify: `crates/macros/tests/koji_resource.rs` (fixtures that reference the fields)
- Modify: `crates/koji-service/tests/*` (any project-CRUD test bodies sending/asserting `golbat`/`api_endpoint`/`api_key` — including the `"golbat": false` bodies added by Tasks 4–8 tests; grep and strip)

**Interfaces:**
- Consumes: Task 1 ran first (data move reads the columns before this drops them — migration order in `lib.rs` guarantees it).
- Produces: `project` table + entity + DTOs are pure grouping (`id`, `name`, `description`, timestamps). `Create<Project>` requires only `name` (+ optional `description`).

- [ ] **Step 1: Write the migration**

```rust
use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_orm::{ConnectionTrait, Statement};

#[derive(DeriveMigrationName)]
pub struct Migration;

async fn has_column(manager: &SchemaManager<'_>, col: &str) -> Result<bool, DbErr> {
    let conn = manager.get_connection();
    let stmt = Statement::from_sql_and_values(
        conn.get_database_backend(),
        "SELECT COLUMN_NAME FROM information_schema.COLUMNS \
         WHERE TABLE_SCHEMA = DATABASE() AND TABLE_NAME = 'project' AND COLUMN_NAME = ? LIMIT 1",
        [col.into()],
    );
    Ok(conn.query_one(stmt).await?.is_some())
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Projects-v2 (spec 2026-07-06): push config now lives in
    /// `webhook_subscription` (migrated by m20260706_000001); the v1 columns are
    /// dead. `golbat` was vestigial in v2 (read-only golbat crate, zero callers).
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        for col in ["api_endpoint", "api_key", "golbat"] {
            if has_column(manager, col).await? {
                log::info!("[MIGRATION] dropping project.{col}");
                manager
                    .get_connection()
                    .execute_unprepared(&format!("ALTER TABLE `project` DROP COLUMN `{col}`"))
                    .await?;
            }
        }
        Ok(())
    }

    /// Restores the columns (empty — push config lives in webhook_subscription
    /// and is not un-migrated).
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if !has_column(manager, "api_endpoint").await? {
            manager
                .get_connection()
                .execute_unprepared(
                    "ALTER TABLE `project` \
                       ADD COLUMN `api_endpoint` VARCHAR(255) NULL, \
                       ADD COLUMN `api_key` VARCHAR(255) NULL, \
                       ADD COLUMN `golbat` TINYINT(1) NOT NULL DEFAULT 0",
                )
                .await?;
        }
        Ok(())
    }
}
```

(Column types in `down` must match what `m20230121_184556_add_project_api.rs` created — read it and copy the exact types.) Register after `m20260706_000001` in `lib.rs`.

- [ ] **Step 2: Strip the fields from code** (compile-error-driven; this is the whole point of doing it in one task)

1. `project.rs` Model: delete `api_endpoint`, `api_key`, `golbat` fields; delete `get_golbat_project()` entirely; in `paginate`'s `json!` block delete the three lines.
2. `utils/json.rs` `to_project`: delete the three field-parsing branches.
3. `resources.rs` project invocation: `create:` block becomes `{ name: String, description: Option<String> }` (+ keep `outbox: true` from Task 6).
4. `cargo build --workspace 2>&1 | grep -E "^error"` — chase every remaining reference (macro test fixtures, service tests, anything grep finds for `api_endpoint|api_key|golbat` in `crates/` excluding koji-golbat's own crate name and `golbat_data` modules — those are the read-side data feature, NOT the project flag; do not touch them).
5. Strip `"golbat": false` from every project-create body in tests (Tasks 4–8 added several).

- [ ] **Step 3: Apply migration + full-workspace verify**

Run (one batch):
- `set -a; source ./.env.test; set +a; DATABASE_URL="$KOJI_DB_URL" cargo run -p migration -- up`
- `cargo build --workspace`
- `set -a; source ./.env.test; set +a; cargo test --workspace` (background it; >5s)

Expected: migration logs 3 drops; build clean; ALL tests green (the project fixture data in `v2_db.rs` etc. no longer mention dropped fields).

- [ ] **Step 4: Commit**

```bash
git add crates/
git commit -m "feat(db)!: drop project.api_endpoint/api_key/golbat — push config lives in webhook_subscription"
```

---

### Task 10: Final verify, spec amendments, memory

**Files:**
- Modify: `docs/superpowers/specs/2026-07-06-projects-webhooks-v2-design.md`
- Modify: `docs/user-stories/v2-parity-gap-list.md` (if it tracks Epic 19/E9.3-adjacent items this plan closed — update honestly, don't over-claim)

- [ ] **Step 1: Full gates, in parallel, backgrounded**

Run in one Bash batch: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets`, `set -a; source ./.env.test; set +a; cargo test --workspace`.
Expected: all green. Fix anything red before proceeding (never advance on red).

- [ ] **Step 2: Amend the spec with as-built deviations**

Add an "As built (2026-07-XX)" section to the spec noting:
- Wire format snake_case (`project_id`), list filter `?project=` (not `?projectId=`) — koji_resource convention.
- `webhook_subscription.id` is `u64` (`BIGINT`, pre-existing); macro update/delete paths take `u32` ids — accepted (`ponytail:` cast, u32 id space is not a real ceiling here).
- Project-PATCH bulk-assign (`"geofences"` key) does not emit membership events yet — deferred to E9.1 (comment marks the spot).
- `/test` returns 200 with `delivered:false` + error detail on upstream failure (only missing webhook is 404).

- [ ] **Step 3: Commit + report**

```bash
git add docs/
git commit -m "docs(spec): projects-webhooks v2 as-built amendments"
```

Report: tasks landed, test counts, any deviations beyond the amended ones.

---

## Self-Review Notes (already applied)

- Spec §3 schema ↔ Task 1 DDL ↔ Task 2/3 entities: field-by-field identical (name/project_id/mode/method/headers; u32 FK; ENUM strings `event|ping`, `GET|POST`).
- Spec §4 five topics ↔ Tasks 6 (project.updated/deleted), 7 (geofence.updated, route.updated), 8 (project.geofences_changed). Matching rule ↔ Task 2 `project_matches` (+ the "no project keys → scoped sub doesn't fire" case made explicit).
- Spec §5 API ↔ Tasks 4-5. Spec §6 migration ↔ Tasks 1 + 9 (split into two migrations so the data move can read columns the second one drops; spec described one migration — same net effect, safer ordering).
- Spec §9 test list ↔ Tasks 1 (parse cases), 2 (matching/ping units), 4 (CRUD+filter+parity canary), 5 (test-fire live listener), 6-8 (emission), 9 (workspace).
- Known judgment calls the reviewer should check: PATCH merge semantics for webhooks (Task 3 Step 4 note), project-PATCH membership emission deferral (Task 8 call-site 2), `crud_query` attribute vs hand-written Query methods (Task 3 Step 1).
