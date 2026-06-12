# DB-managed plugin config — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make plugin operational config (enabled / default-args / description) manageable from the admin panel via `/api/v2/plugins`, backed by a DB overlay, while the executable declaration stays disk-owned.

**Architecture:** Disk `plugin.toml` remains the trusted manifest. A new `plugin_config` table is a management *overlay* keyed by `(kind, name)`. The plugin registry moves from algorithms' fixed `LazyLock` into an installable process-global in `koji-plugins` (`install`/`current`); `koji-service` builds it from disk ∪ overlay at startup and rebuilds it after each write.

**Tech Stack:** Rust, sea-orm 1.1 (sqlx-mysql) migrations + entities, actix-web 4, `ApiResponse` (`ok`/`error`) envelope, react-admin (client).

**Spec:** `docs/superpowers/specs/2026-06-11-koji-plugin-config-db-design.md`

**Codebase test reality (read before starting):**
- `koji-db` has **no** in-crate DB test harness (no `MockDatabase`); entity `Query` methods are verified by downstream runtime smoke, not unit tests.
- `koji-service` has **no** actix integration-test harness; handlers are verified by runtime smoke (boot the server against the dev DB + `curl`), as the rest of the v2 surface was.
- **Pure-logic** code (registry overlay/merge, id parsing, JSON arg merge) **does** get real in-crate `#[test]`/`#[tokio::test]` units — those are the TDD tasks.

**Build/run notes:**
- Whole-workspace build: `cargo build` (the server package is **`koji`**, never `-p koji-server`).
- Dev server boot (auto-applies migrations): `set -a; . ./.env; set +a; unset CONTROLLER_DB_URL; PORT=8080 HOST=127.0.0.1 LOG_LEVEL=warn ./target/debug/koji &` then poll `http://127.0.0.1:8080/healthz`. Auth header for v2: `-H "Authorization: Bearer secret"` (dev `KOJI_SECRET`).
- The `mysql` CLI cannot auth against the dev server (native_password plugin removed) — do **not** use it; verify schema/data through the API or `cargo run -p migration -- status`.

**Commit after every task** (project CLAUDE.md: commit freely; end messages with `Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>`). Do **not** push (branch stays local until the security rework).

---

### Task 1: Migration — `plugin_config` table

**Files:**
- Create: `crates/migration/src/m20260529_000004_create_plugin_config.rs`
- Modify: `crates/migration/src/lib.rs` (register the migration)

- [ ] **Step 1: Write the migration** (mirrors `m20230301_051446_tile_server_table.rs`; adds `kind`, `enabled`, `args_default`, `description` + a unique `(kind,name)` index)

```rust
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        log::info!("[MIGRATION] creating plugin_config table");
        manager
            .create_table(
                Table::create()
                    .table(PluginConfig::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(PluginConfig::Id)
                            .integer()
                            .not_null()
                            .unsigned()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(PluginConfig::Name).string().not_null())
                    .col(ColumnDef::new(PluginConfig::Kind).string().not_null())
                    .col(
                        ColumnDef::new(PluginConfig::Enabled)
                            .boolean()
                            .not_null()
                            .default(true),
                    )
                    .col(ColumnDef::new(PluginConfig::ArgsDefault).json().null())
                    .col(ColumnDef::new(PluginConfig::Description).string().null())
                    .col(
                        ColumnDef::new(PluginConfig::CreatedAt)
                            .timestamp()
                            .not_null()
                            .extra("DEFAULT CURRENT_TIMESTAMP".to_string()),
                    )
                    .col(
                        ColumnDef::new(PluginConfig::UpdatedAt)
                            .timestamp()
                            .not_null()
                            .extra("DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP".to_string()),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_plugin_config_kind_name")
                    .table(PluginConfig::Table)
                    .col(PluginConfig::Kind)
                    .col(PluginConfig::Name)
                    .unique()
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        log::info!("[MIGRATION] dropping plugin_config table");
        manager
            .drop_table(Table::drop().table(PluginConfig::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
enum PluginConfig {
    Table,
    Id,
    Name,
    Kind,
    Enabled,
    ArgsDefault,
    Description,
    CreatedAt,
    UpdatedAt,
}
```

- [ ] **Step 2: Register the migration** in `crates/migration/src/lib.rs` — add `mod m20260529_000004_create_plugin_config;` with the other `mod` lines, and `Box::new(m20260529_000004_create_plugin_config::Migration),` as the **last** entry of the `migrations()` vec (after `m20260529_000003_dragonite_linkage`).

- [ ] **Step 3: Build**

Run: `cargo build -p migration`
Expected: `Finished` (compiles).

- [ ] **Step 4: Apply + verify** against the dev DB

Run: `set -a; . ./.env; set +a; cargo run -p migration -- status` then `cargo run -p migration -- up`
Expected: `status` lists `m20260529_000004_create_plugin_config` as Pending, then `up` applies it (`Applying ... plugin_config`). Re-run `status` → it shows Applied.

- [ ] **Step 5: Commit**

```bash
git add crates/migration/src/m20260529_000004_create_plugin_config.rs crates/migration/src/lib.rs
git commit -m "feat(migration): plugin_config overlay table (kind,name unique)"
```

---

### Task 2: koji-db entity — `plugin_config`

**Files:**
- Create: `crates/koji-db/src/db/plugin_config.rs`
- Modify: `crates/koji-db/src/db/mod.rs` (add `pub mod plugin_config;`), `crates/koji-db/src/db/prelude.rs` (re-export if that file re-exports the others — match the existing pattern)

(No in-crate test — koji-db has no DB test harness; the `Query` is exercised by Task 5's runtime smoke.)

- [ ] **Step 1: Write the entity + Query** (mirrors `tile_server.rs` Model; `Query` is overlay-shaped, keyed by `(kind,name)`)

```rust
use super::*;

use sea_orm::entity::prelude::*;
use sea_orm::{ActiveValue::Set, IntoActiveModel};
use serde_json::json;

use crate::error::ModelError;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "plugin_config")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: u32,
    pub name: String,
    pub kind: String,
    pub enabled: bool,
    #[sea_orm(column_type = "Json", nullable)]
    pub args_default: Option<Json>,
    pub description: Option<String>,
    pub created_at: DateTimeUtc,
    pub updated_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

pub struct Query;

impl Query {
    /// All overlay rows (feeds the registry build).
    pub async fn all(db: &DatabaseConnection) -> Result<Vec<Model>, ModelError> {
        Ok(Entity::find().all(db).await?)
    }

    /// One overlay row by `(kind, name)`, if present.
    pub async fn get_one(
        db: &DatabaseConnection,
        kind: &str,
        name: &str,
    ) -> Result<Option<Model>, ModelError> {
        Ok(Entity::find()
            .filter(Column::Kind.eq(kind))
            .filter(Column::Name.eq(name))
            .one(db)
            .await?)
    }

    /// Insert or update the overlay for `(kind, name)`. `None` fields leave the
    /// stored value unchanged on update / fall to column defaults on insert.
    pub async fn upsert(
        db: &DatabaseConnection,
        kind: &str,
        name: &str,
        enabled: Option<bool>,
        args_default: Option<Json>,
        description: Option<String>,
    ) -> Result<Model, ModelError> {
        let existing = Query::get_one(db, kind, name).await?;
        let mut active = match existing {
            Some(model) => model.into_active_model(),
            None => ActiveModel {
                kind: Set(kind.to_string()),
                name: Set(name.to_string()),
                ..Default::default()
            },
        };
        if let Some(enabled) = enabled {
            active.enabled = Set(enabled);
        }
        if let Some(args) = args_default {
            active.args_default = Set(Some(args));
        }
        if let Some(desc) = description {
            active.description = Set(Some(desc));
        }
        let model = active.save(db).await?;
        // Read back the full row (save() returns the active model).
        Query::get_one(db, kind, name)
            .await?
            .ok_or_else(|| ModelError::Geofence("plugin_config upsert read-back failed".to_string()))
    }

    pub async fn get_one_json(
        db: &DatabaseConnection,
        kind: &str,
        name: &str,
    ) -> Result<Json, ModelError> {
        Ok(json!(Query::get_one(db, kind, name).await?))
    }

    /// Delete the overlay for `(kind, name)` (reset to defaults).
    pub async fn delete(
        db: &DatabaseConnection,
        kind: &str,
        name: &str,
    ) -> Result<sea_orm::DeleteResult, ModelError> {
        Ok(Entity::delete_many()
            .filter(Column::Kind.eq(kind))
            .filter(Column::Name.eq(name))
            .exec(db)
            .await?)
    }
}
```

> Note: `ModelError::Geofence` is reused for the read-back error to match the existing `tile_server` style (its `get_one` does the same). If a more general variant exists in `crate::error`, prefer it — check `crates/koji-db/src/error.rs` first.

- [ ] **Step 2: Register the module** — add `pub mod plugin_config;` to `crates/koji-db/src/db/mod.rs` alongside the other `pub mod` lines; if `db/prelude.rs` re-exports each entity, add `plugin_config` there in the same style.

- [ ] **Step 3: Build**

Run: `cargo build -p koji-db`
Expected: `Finished`. (Fix any `into_active_model`/`ActiveValue` import per the compiler; `tile_server.rs` shows the canonical imports.)

- [ ] **Step 4: Commit**

```bash
git add crates/koji-db/src/db/plugin_config.rs crates/koji-db/src/db/mod.rs crates/koji-db/src/db/prelude.rs
git commit -m "feat(db): plugin_config entity + overlay Query (all/get_one/upsert/delete)"
```

---

### Task 3: koji-plugins — installable registry + overlay (TDD)

**Files:**
- Create: `crates/koji-plugins/src/global.rs`
- Modify: `crates/koji-plugins/src/registry.rs` (overlay map + accessors), `crates/koji-plugins/src/lib.rs` (export `install`/`current`, `Overlay`)
- Test: inline `#[cfg(test)]` in `registry.rs` + `global.rs`

- [ ] **Step 1: Write failing tests** for the overlay in `registry.rs` (append to its `#[cfg(test)] mod tests`)

```rust
#[test]
fn overlay_disables_entry_from_names_and_get() {
    let mut reg = PluginRegistry::default();
    reg.insert_manifest_for_test(PluginKind::Routing, "tsp");
    assert!(reg.names(PluginKind::Routing).contains(&"tsp".to_string()));
    reg.set_overlay(PluginKind::Routing, "tsp", Overlay { enabled: false, args_default: None });
    assert!(!reg.names(PluginKind::Routing).contains(&"tsp".to_string()), "disabled hidden from names");
    assert!(reg.get(PluginKind::Routing, "tsp").is_none(), "disabled get -> None");
    assert!(!reg.is_enabled(PluginKind::Routing, "tsp"));
}

#[test]
fn no_overlay_defaults_to_enabled() {
    let mut reg = PluginRegistry::default();
    reg.insert_manifest_for_test(PluginKind::Clustering, "kmeans");
    assert!(reg.is_enabled(PluginKind::Clustering, "kmeans"));
    assert!(reg.get(PluginKind::Clustering, "kmeans").is_some());
}

#[test]
fn overlay_surfaces_args_default() {
    let mut reg = PluginRegistry::default();
    reg.insert_manifest_for_test(PluginKind::Clustering, "kmeans");
    reg.set_overlay(
        PluginKind::Clustering,
        "kmeans",
        Overlay { enabled: true, args_default: Some(serde_json::json!({"k": 8})) },
    );
    assert_eq!(reg.args_default(PluginKind::Clustering, "kmeans"), Some(&serde_json::json!({"k": 8})));
}
```

> `insert_manifest_for_test` is a `#[cfg(test)]` helper you add on `PluginRegistry` that inserts a minimal `PluginManifest` (name, kind, `entrypoint: "x"`, defaults) + a dummy dir into the `manifests`/`dirs` maps, so the overlay tests don't need real files.

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p koji-plugins overlay`
Expected: FAIL — `Overlay`, `set_overlay`, `is_enabled`, `args_default`, `insert_manifest_for_test` not found.

- [ ] **Step 3: Implement the overlay** in `registry.rs`

```rust
use serde_json::Value;

/// Per-plugin management overlay (from the DB `plugin_config` table).
#[derive(Debug, Clone)]
pub struct Overlay {
    pub enabled: bool,
    pub args_default: Option<Value>,
}

impl Default for Overlay {
    fn default() -> Self {
        Overlay { enabled: true, args_default: None }
    }
}
```

Add to `struct PluginRegistry`: `overlays: HashMap<(PluginKind, String), Overlay>,` (extend `#[derive(Default)]` coverage — `HashMap` is `Default`).

Add methods to `impl PluginRegistry`:

```rust
/// Apply a management overlay for `(kind, name)`.
pub fn set_overlay(&mut self, kind: PluginKind, name: &str, overlay: Overlay) {
    self.overlays.insert((kind, name.to_string()), overlay);
}

/// Whether `(kind, name)` is enabled (default `true` when no overlay).
pub fn is_enabled(&self, kind: PluginKind, name: &str) -> bool {
    self.overlays
        .get(&(kind, name.to_string()))
        .map_or(true, |o| o.enabled)
}

/// The overlay default-args for `(kind, name)`, if any.
pub fn args_default(&self, kind: PluginKind, name: &str) -> Option<&Value> {
    self.overlays
        .get(&(kind, name.to_string()))
        .and_then(|o| o.args_default.as_ref())
}

/// The manifest for `(kind, name)` **ignoring** the enabled overlay — for the
/// admin view, which must show disabled plugins too. (Runtime dispatch uses
/// the enabled-gated [`get`](Self::get) instead.)
pub fn manifest_unfiltered(&self, kind: PluginKind, name: &str) -> Option<&PluginManifest> {
    self.manifests.get(&(kind, name.to_string()))
}

/// The plugin directory for `(kind, name)` (for `Plugin::from_manifest`).
pub fn dir(&self, kind: PluginKind, name: &str) -> Option<&std::path::Path> {
    self.dirs.get(&(kind, name.to_string())).map(|p| p.as_path())
}

/// Every discovered `(kind, name)` (enabled or not) — for the admin list.
pub fn all_unfiltered_keys(&self) -> Vec<(PluginKind, String)> {
    self.manifests.keys().cloned().collect()
}

#[cfg(test)]
pub(crate) fn insert_manifest_for_test(&mut self, kind: PluginKind, name: &str) {
    let manifest = PluginManifest {
        name: name.to_string(),
        kind,
        entrypoint: "x".to_string(),
        interpreter: None,
        version: None,
        description: None,
        protocol: Default::default(),
    };
    let key = (kind, name.to_string());
    self.dirs.insert(key.clone(), std::path::PathBuf::from("."));
    self.manifests.insert(key, manifest);
}
```

Modify the existing `names(kind)` to filter enabled, and `get(kind, name)` to gate on enabled. Find them in `registry.rs` and make:

```rust
pub fn names(&self, kind: PluginKind) -> Vec<String> {
    self.manifests
        .keys()
        .filter(|(k, _)| *k == kind)
        .filter(|(k, n)| self.is_enabled(*k, n))
        .map(|(_, n)| n.clone())
        .collect()
}

pub fn get(&self, kind: PluginKind, name: &str) -> Option<&PluginManifest> {
    if !self.is_enabled(kind, name) {
        return None;
    }
    self.manifests.get(&(kind, name.to_string()))
}
```

> Match the actual current signatures of `names`/`get` in `registry.rs` (they already exist — this only adds the enabled filter). Keep the `dirs` lookup used by `Plugin` resolution intact.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p koji-plugins overlay`
Expected: PASS (3 tests).

- [ ] **Step 5: Write the installable global** `crates/koji-plugins/src/global.rs`

```rust
//! Process-global, replaceable plugin registry.
//!
//! Defaults to a lazy disk scan (`PluginRegistry::from_env`) for back-compat;
//! `koji-service` replaces it via [`install`] once it has merged the DB overlay.

use std::sync::{Arc, OnceLock, RwLock};

use crate::registry::PluginRegistry;

fn cell() -> &'static RwLock<Arc<PluginRegistry>> {
    static REGISTRY: OnceLock<RwLock<Arc<PluginRegistry>>> = OnceLock::new();
    REGISTRY.get_or_init(|| RwLock::new(Arc::new(PluginRegistry::from_env())))
}

/// Replace the process-global registry (called at startup + after each write).
pub fn install(registry: PluginRegistry) {
    *cell().write().expect("registry lock poisoned") = Arc::new(registry);
}

/// A snapshot of the current registry (cheap `Arc` clone).
pub fn current() -> Arc<PluginRegistry> {
    Arc::clone(&cell().read().expect("registry lock poisoned"))
}
```

- [ ] **Step 6: Export** from `crates/koji-plugins/src/lib.rs` — add `pub mod global;` and re-export: `pub use global::{current, install};` and `pub use registry::{Overlay, PluginRegistry};` (add `Overlay` to the existing registry re-export).

- [ ] **Step 7: Add a global round-trip test** to `global.rs`

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{PluginKind, registry::Overlay};

    #[test]
    fn install_then_current_reflects_replacement() {
        let mut reg = PluginRegistry::default();
        reg.insert_manifest_for_test(PluginKind::Routing, "demo");
        reg.set_overlay(PluginKind::Routing, "demo", Overlay { enabled: false, args_default: None });
        install(reg);
        assert!(!current().is_enabled(PluginKind::Routing, "demo"));
    }
}
```

- [ ] **Step 8: Run + build**

Run: `cargo test -p koji-plugins`
Expected: PASS (existing 22 + the 4 new). `cargo build` whole-workspace green.

- [ ] **Step 9: Commit**

```bash
git add crates/koji-plugins/src
git commit -m "feat(plugins): installable registry global (install/current) + (kind,name) overlay"
```

---

### Task 4: algorithms — read `koji_plugins::current()` + merge overlay args

**Files:**
- Modify: `crates/algorithms/src/plugins.rs`
- Test: inline `#[cfg(test)]` in `plugins.rs`

- [ ] **Step 1: Write a failing test** (append to / create `#[cfg(test)] mod tests` in `plugins.rs`)

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_args_request_overrides_default() {
        let default = serde_json::json!({"k": 8, "iters": 50});
        let request = serde_json::json!({"k": 12});
        assert_eq!(
            merge_args(Some(&default), request),
            serde_json::json!({"k": 12, "iters": 50})
        );
    }

    #[test]
    fn merge_args_null_request_falls_back_to_default() {
        let default = serde_json::json!({"k": 8});
        assert_eq!(merge_args(Some(&default), serde_json::Value::Null), serde_json::json!({"k": 8}));
    }

    #[test]
    fn merge_args_no_default_returns_request() {
        let request = serde_json::json!({"k": 12});
        assert_eq!(merge_args(None, request.clone()), request);
    }
}
```

> Only `merge_args` is unit-tested here — it's pure. The disabled-plugin `resolve` → `None` behavior is already covered in Task 3 (koji-plugins), and re-testing it from `algorithms` would need a cross-crate test helper (`insert_manifest_for_test` is `#[cfg(test)]`-private to koji-plugins). Don't add a test-util feature for this — the coverage already exists.

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test -p algorithms merge_args`
Expected: FAIL — `merge_args` not found.

- [ ] **Step 3: Implement** — in `plugins.rs`, delete the `static REGISTRY: LazyLock<...>` + `registry()` fn, and add `merge_args` + route reads through `koji_plugins::current()`:

```rust
use serde_json::Value;

/// Merge overlay default args with request args; request keys win. A `null`
/// request falls back to the defaults; non-object operands let the request win.
pub(crate) fn merge_args(default: Option<&Value>, request: Value) -> Value {
    match (default, &request) {
        (Some(Value::Object(d)), Value::Object(r)) => {
            let mut merged = d.clone();
            for (k, v) in r {
                merged.insert(k.clone(), v.clone());
            }
            Value::Object(merged)
        }
        (Some(d), Value::Null) => d.clone(),
        _ => request,
    }
}
```

Update `plugin_names` and `resolve` to use the global:

```rust
pub(crate) fn plugin_names(kind: PluginKind) -> Vec<String> {
    koji_plugins::current().names(kind)
}

pub(crate) fn resolve(kind: PluginKind, name: &str, split_level: u64) -> Option<Plugin> {
    let registry = koji_plugins::current();
    let manifest = registry.get(kind, name)?; // None when disabled or unknown
    let dir = registry.dir(kind, name)?;      // existing dir lookup
    Plugin::from_manifest(manifest, dir, split_level).ok()
}
```

> Match the real current bodies of `resolve`/`plugin_names` (the dir lookup may be named differently — keep whatever `registry().get(...)`/dir accessor exists, now via `koji_plugins::current()`). The `Arc<PluginRegistry>` from `current()` lives for the function scope, so the borrows are fine.

- [ ] **Step 4: Wire the arg-merge at the Custom dispatch sites.** Find where the `Custom(name)` arms call the plugin with `args_to_value(plugin_args)` (clustering / routing / bootstrap). Wrap the request args:

```rust
let request_args = args_to_value(plugin_args);
let args = merge_args(koji_plugins::current().args_default(kind, name), request_args);
```

Pass `args` to `plugin.run_multi(...)`. Apply at each of the (≤3) Custom call sites.

- [ ] **Step 5: Run + build**

Run: `cargo test -p algorithms` then `cargo build`
Expected: PASS (3 `merge_args` tests); workspace builds.

- [ ] **Step 6: Commit**

```bash
git add crates/algorithms/src/plugins.rs crates/algorithms/Cargo.toml
git commit -m "refactor(algorithms): read installable plugin registry; merge overlay default args"
```

---

### Task 5: koji-service — `/api/v2/plugins` + rebuild/install wiring

**Files:**
- Modify: `crates/koji-service/Cargo.toml` (add `koji-plugins`)
- Create: `crates/koji-service/src/public/v2/plugins.rs`
- Modify: `crates/koji-service/src/public/v2/mod.rs` (`pub mod plugins;`), `crates/koji-service/src/lib.rs` (startup install + scope wiring)
- Test: inline `#[cfg(test)]` in `plugins.rs` (id parsing only); the rest is runtime smoke.

- [ ] **Step 1: Add the dep** to `crates/koji-service/Cargo.toml`: `koji-plugins = { path = "../koji-plugins" }` (alongside `koji-db`).

- [ ] **Step 2: Write a failing test** for the id parser in `plugins.rs`

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_id_splits_kind_and_name() {
        assert_eq!(parse_id("routing:tsp").unwrap(), (PluginKind::Routing, "tsp".to_string()));
        assert_eq!(parse_id("clustering:k:means").unwrap(), (PluginKind::Clustering, "k:means".to_string()));
    }

    #[test]
    fn parse_id_rejects_missing_colon_and_bad_kind() {
        assert!(parse_id("tsp").is_none());
        assert!(parse_id("teleport:x").is_none());
    }
}
```

- [ ] **Step 3: Run to verify it fails**

Run: `cargo test -p koji-service parse_id`
Expected: FAIL — `parse_id` not found.

- [ ] **Step 4: Implement `plugins.rs`**

```rust
//! v2 plugin management — `/api/v2/plugins` (architecture: DB-managed plugin
//! config). Reads the merged disk-manifest + DB-overlay view; PATCH/DELETE edit
//! only the overlay (`enabled`/`args_default`/`description`). The executable
//! declaration (`entrypoint`/`interpreter`/`protocol`) is disk-owned + read-only.
//!
//! Mounted under the shared `public_validator` like the other v2 resources; a
//! dedicated gate is deferred to the global API-security rework.

use actix_web::{Error, HttpResponse, http::StatusCode, web};
use koji_db::{KojiDb, db::plugin_config};
use koji_plugins::{PluginKind, PluginRegistry};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::utils::api_response::ApiResponse;

/// Parse a `"{kind}:{name}"` resource id. `name` may itself contain `:`.
fn parse_id(id: &str) -> Option<(PluginKind, String)> {
    let (kind, name) = id.split_once(':')?;
    let kind = match kind {
        "clustering" => PluginKind::Clustering,
        "routing" => PluginKind::Routing,
        "bootstrap" => PluginKind::Bootstrap,
        _ => return None,
    };
    if name.is_empty() {
        return None;
    }
    Some((kind, name.to_string()))
}

/// PATCH body — every field optional (overlay merge).
#[derive(Debug, Deserialize)]
struct PluginPatch {
    enabled: Option<bool>,
    args_default: Option<Value>,
    description: Option<String>,
}

/// Build the merged view (disk manifest + overlay) for one plugin from the
/// current registry, as the JSON the API returns.
fn plugin_view(reg: &PluginRegistry, kind: PluginKind, name: &str) -> Option<Value> {
    let manifest = reg.manifest_unfiltered(kind, name)?; // see note below
    Some(json!({
        "id": format!("{kind}:{name}"),
        "name": name,
        "kind": kind.to_string(),
        "entrypoint": manifest.entrypoint,
        "interpreter": manifest.interpreter,
        "protocol": format!("{:?}", manifest.protocol).to_lowercase(),
        "version": manifest.version,
        "description": manifest.description,
        "enabled": reg.is_enabled(kind, name),
        "args_default": reg.args_default(kind, name),
    }))
}

/// Rebuild the registry from disk ∪ DB overlay and install it process-wide.
pub async fn rebuild_and_install(db: &KojiDb) -> Result<(), Error> {
    let mut reg = PluginRegistry::from_env();
    let rows = plugin_config::Query::all(&db.koji)
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;
    for row in rows {
        if let Some((kind, name)) = parse_id(&format!("{}:{}", row.kind, row.name)) {
            reg.set_overlay(
                kind,
                &name,
                koji_plugins::Overlay { enabled: row.enabled, args_default: row.args_default },
            );
        }
    }
    koji_plugins::install(reg);
    Ok(())
}

async fn list(_db: web::Data<KojiDb>) -> Result<HttpResponse, Error> {
    let reg = koji_plugins::current();
    let views: Vec<Value> = reg
        .all_unfiltered_keys() // (kind, name) for every disk-discovered plugin
        .into_iter()
        .filter_map(|(kind, name)| plugin_view(&reg, kind, &name))
        .collect();
    Ok(ApiResponse::success(views))
}

async fn get_one(path: web::Path<String>) -> Result<HttpResponse, Error> {
    let id = path.into_inner();
    let Some((kind, name)) = parse_id(&id) else {
        return Ok(ApiResponse::fail(StatusCode::BAD_REQUEST, json!({ "id": "expected {kind}:{name}" })));
    };
    let reg = koji_plugins::current();
    match plugin_view(&reg, kind, &name) {
        Some(view) => Ok(ApiResponse::success(view)),
        None => Ok(ApiResponse::fail(StatusCode::NOT_FOUND, json!({ "id": format!("no plugin {id}") }))),
    }
}

async fn update(
    db: web::Data<KojiDb>,
    path: web::Path<String>,
    body: web::Json<PluginPatch>,
) -> Result<HttpResponse, Error> {
    let id = path.into_inner();
    let Some((kind, name)) = parse_id(&id) else {
        return Ok(ApiResponse::fail(StatusCode::BAD_REQUEST, json!({ "id": "expected {kind}:{name}" })));
    };
    // Enforce the disk gate: only configure a plugin that exists on disk.
    if koji_plugins::current().manifest_unfiltered(kind, &name).is_none() {
        return Ok(ApiResponse::fail(
            StatusCode::UNPROCESSABLE_ENTITY,
            json!({ "id": format!("no installed plugin {id}; drop a plugin.toml first") }),
        ));
    }
    let patch = body.into_inner();
    plugin_config::Query::upsert(&db.koji, &kind.to_string(), &name, patch.enabled, patch.args_default, patch.description)
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;
    rebuild_and_install(&db).await?;
    let reg = koji_plugins::current();
    Ok(ApiResponse::success(plugin_view(&reg, kind, &name)))
}

async fn remove(db: web::Data<KojiDb>, path: web::Path<String>) -> Result<HttpResponse, Error> {
    let id = path.into_inner();
    let Some((kind, name)) = parse_id(&id) else {
        return Ok(ApiResponse::fail(StatusCode::BAD_REQUEST, json!({ "id": "expected {kind}:{name}" })));
    };
    let result = plugin_config::Query::delete(&db.koji, &kind.to_string(), &name)
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;
    rebuild_and_install(&db).await?;
    Ok(ApiResponse::success(json!({ "rows_affected": result.rows_affected })))
}

pub fn scope() -> actix_web::Scope {
    web::scope("/plugins")
        .service(web::resource("").route(web::get().to(list)))
        .service(
            web::resource("/{id}")
                .route(web::get().to(get_one))
                .route(web::patch().to(update))
                .route(web::delete().to(remove)),
        )
}
```

> **Registry accessors used here** (`manifest_unfiltered`, `dir`, `all_unfiltered_keys`) are defined in **Task 3 Step 3** — the enabled-gated `get`/`names` serve the runtime path; the `*_unfiltered` variants serve this admin view. `koji_plugins::Overlay` is re-exported in Task 3 Step 6.

- [ ] **Step 5: Run the id test**

Run: `cargo test -p koji-service parse_id`
Expected: PASS (2 tests).

- [ ] **Step 6: Wire startup install + scope** in `crates/koji-service/src/lib.rs`:
  - In `start()`, after `KojiDb` is constructed and before/at server build, call the rebuild once: `public::v2::plugins::rebuild_and_install(&db).await.ok();` (so the registry reflects the DB overlay from boot; `.ok()` — a failure just leaves the lazy disk default).
  - Add `.service(public::v2::plugins::scope())` to the `/api/v2` service list (next to `geo`/`scanner_data`).
  - Add `pub mod plugins;` to `crates/koji-service/src/public/v2/mod.rs`.

- [ ] **Step 7: Build + runtime smoke** (no actix test harness — verify live)

Run: `cargo build` then boot the dev server (see header). Then:
```bash
A='-H "Authorization: Bearer secret"'
# list — expect tsp present, enabled:true, entrypoint read-only
curl -s "http://127.0.0.1:8080/api/v2/plugins" -H "Authorization: Bearer secret"
# disable tsp
curl -s -X PATCH "http://127.0.0.1:8080/api/v2/plugins/routing:tsp" -H "Authorization: Bearer secret" -H 'Content-Type: application/json' -d '{"enabled":false}'
# tsp now absent from meta/algorithms.routing
curl -s "http://127.0.0.1:8080/api/v2/meta/algorithms" -H "Authorization: Bearer secret"
# unknown plugin -> 422 ; malformed id -> 400
curl -s -X PATCH "http://127.0.0.1:8080/api/v2/plugins/routing:nope" -H "Authorization: Bearer secret" -d '{"enabled":false}'
curl -s -X PATCH "http://127.0.0.1:8080/api/v2/plugins/nocolon" -H "Authorization: Bearer secret" -d '{"enabled":false}'
# reset
curl -s -X DELETE "http://127.0.0.1:8080/api/v2/plugins/routing:tsp" -H "Authorization: Bearer secret"
```
Expected: list `status:ok` with a `tsp` row; PATCH → `enabled:false`; `meta/algorithms.routing` drops `tsp`; unknown → `status:error code:unprocessable`; malformed → `status:error code:invalid_request`; DELETE → `rows_affected:1` and `tsp` re-appears in meta.

> Note: in the local (non-Docker) dev env the `tsp` entrypoint (`/algorithms/.../tsp`) does not exist, so `PluginRegistry::from_env` may skip it (entrypoint-missing). If `tsp` is absent from the list locally, temporarily add a throwaway `plugins/demo/plugin.toml` (`kind="clustering"`, `entrypoint="x.py"` won't resolve either) — instead, assert the smoke against the *list/PATCH/422/400* behavior using whatever the scan discovers, and confirm the 422 path (unknown plugin) which needs no on-disk plugin. The merge/disable logic is unit-covered in Tasks 3–4; this smoke confirms wiring + envelope.

- [ ] **Step 8: Commit**

```bash
git add crates/koji-service/Cargo.toml crates/koji-service/src/public/v2/plugins.rs crates/koji-service/src/public/v2/mod.rs crates/koji-service/src/lib.rs
git commit -m "feat(service): /api/v2/plugins overlay CRUD + registry rebuild-on-write"
```

---

### Task 6: Client — react-admin `plugins` Resource (bare minimum, throwaway)

**Files:**
- Modify: the react-admin resource registration (find via `grep -rn "<Resource" client/src`) — typically `client/src/App.tsx` or a resources index.
- Create: `client/src/pages/admin/plugins.tsx` (or co-locate per the existing resource file convention — match how `tile-servers`/`projects` resources are defined).

> Keep this minimal — the client is being rewritten. No tests, no polish.

- [ ] **Step 1: Find the existing resource pattern**

Run: `grep -rn "<Resource" client/src; grep -rln "tile" client/src/pages client/src/components`
Read one existing resource (e.g. tile-servers) to copy its List/Edit shape + data-provider wiring.

- [ ] **Step 2: Add the `plugins` resource** mirroring that pattern:

```tsx
import { List, Datagrid, TextField, BooleanField, Edit, SimpleForm, BooleanInput, TextInput } from 'react-admin';

export const PluginList = () => (
  <List>
    <Datagrid rowClick="edit">
      <TextField source="name" />
      <TextField source="kind" />
      <BooleanField source="enabled" />
      <TextField source="version" />
    </Datagrid>
  </List>
);

export const PluginEdit = () => (
  <Edit>
    <SimpleForm>
      {/* read-only manifest fields */}
      <TextInput source="entrypoint" disabled />
      <TextInput source="interpreter" disabled />
      <TextInput source="protocol" disabled />
      {/* editable overlay */}
      <BooleanInput source="enabled" />
      <TextInput source="args_default" multiline parse={(v: string) => { try { return JSON.parse(v); } catch { return v; } }} format={(v: unknown) => (typeof v === 'string' ? v : JSON.stringify(v ?? null))} />
      <TextInput source="description" />
    </SimpleForm>
  </Edit>
);
```

- [ ] **Step 3: Register** `<Resource name="plugins" list={PluginList} edit={PluginEdit} />` in the resources list (no `create` — new plugins come from disk). Ensure the data provider maps `plugins` → `/api/v2/plugins` with id `{kind}:{name}` (the API returns `id`, react-admin uses it directly).

- [ ] **Step 4: Build the client** to confirm it compiles

Run: `cd client && npm run build` (or the project's build script — check `client/package.json`)
Expected: build succeeds.

- [ ] **Step 5: Commit**

```bash
git add client/src
git commit -m "feat(client): minimal react-admin plugins resource (overlay CRUD; throwaway)"
```

---

### Task 7: Docs + final verification

**Files:**
- Modify: `crates/koji-service/openapi.yaml` (add `/api/v2/plugins` paths + a `Plugin` schema + `plugins` tag)
- Modify: `refactor-workspace/roadmap.md` (Phase 9 entry)

- [ ] **Step 1: Add the OpenAPI paths + schema.** Add a `plugins` tag; add `GET /api/v2/plugins`, `GET|PATCH|DELETE /api/v2/plugins/{id}` (responses ref `ApiOk`/`BadRequest`/`Unprocessable`); add a `Plugin` schema (`id, name, kind, entrypoint, interpreter, protocol, version, enabled, args_default, description` — note entrypoint/interpreter/protocol/version are read-only) and a `PluginPatch` request schema (`enabled?, args_default?, description?`). Mirror the style of the existing paths/schemas.

- [ ] **Step 2: Validate the OpenAPI**

Run: `ruby -ryaml -E UTF-8 -e 'YAML.load_file("crates/koji-service/openapi.yaml"); puts "ok"'` then a ref-resolution check (reuse the validation snippet from the conformance work).
Expected: parses; all `$ref`s resolve.

- [ ] **Step 3: Full-suite verification**

Run (parallel): `cargo test --workspace` · `cargo clippy --workspace` · `cargo fmt --all --check`
Expected: all tests pass; 0 clippy errors; fmt clean. (If clippy/fmt touch new files, `cargo fmt --all` + re-stage.)

- [ ] **Step 4: Roadmap entry** — add a "Phase 9 — DB-managed plugin config" section to `refactor-workspace/roadmap.md` summarizing the commits + the overlay model + the deferred items (Model A, `[args]` schema, auth gate).

- [ ] **Step 5: Commit**

```bash
git add crates/koji-service/openapi.yaml refactor-workspace/roadmap.md
git commit -m "docs: OpenAPI /plugins paths + roadmap Phase 9 (DB-managed plugin config)"
```

---

## Notes for the implementer
- **Layering invariant:** `algorithms` must NOT gain a `koji-db` dependency. The DB→registry bridge lives entirely in `koji-service` (`rebuild_and_install`); `algorithms` only reads `koji_plugins::current()`.
- **Registry accessors** (`manifest_unfiltered`, `all_unfiltered_keys`, `dir`) referenced in Task 5 must exist on `PluginRegistry` — add any missing ones in Task 3 (trivial map reads). Keep the `enabled`-filtered `get`/`names` for the *runtime* path and the `*_unfiltered` variants for the *admin view*.
- **`PluginProtocol` → string:** the view renders `protocol` via lowercased debug; if `PluginProtocol` already has a `Serialize`/`Display`, prefer that for stability.
- **No push.** Branch stays local (security rework pending).
```
