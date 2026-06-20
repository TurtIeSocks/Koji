# Import Wizard — Phase A (backend) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add an atomic bulk-import backend — `POST /internal/import` (one DB transaction, dedupe/upsert by name, server-side route-parent resolution, per-feature result map, dry-run preview) — plus an `/internal/geometry/convert` alias.

**Architecture:** First make the koji-db write layer transaction-capable by widening the geofence/route/property/project write fns from `&DatabaseConnection` to generic `&C where C: ConnectionTrait` (so they run on a `DatabaseTransaction`) and serializing the two `future::try_join_all` write fan-outs (concurrent statements on one txn connection are unsound). Then add a koji-db `import` module that owns the dry-run validation + the `db.begin()`→write→`commit`/`rollback` orchestration, and a thin service handler that maps the HTTP DTO to it.

**Tech Stack:** Rust, actix-web, SeaORM 0.10-era (`ConnectionTrait`/`TransactionTrait`/`DatabaseTransaction`), serde_json. Crates: `koji-db` (entity/query layer), `koji-service` (HTTP). Bin package = `koji`.

## Global Constraints

- The admin client calls `/internal/*` ONLY. Both new endpoints mount under the `/internal` scope (`crates/koji-service/src/internal/mod.rs`), session-authed by the existing `public_validator` middleware. Do NOT add them only under `/api/v2`.
- Widening a fn to `&C where C: ConnectionTrait` is **backward-compatible**: every existing caller passes `&conn.koji` (a `&DatabaseConnection`), which satisfies `&impl ConnectionTrait`. Do NOT touch call sites.
- Inside a transaction, statements must be **sequential** — replace every `future::try_join_all(... write ...)` in a widened fn with a `for` loop. Leave concurrent fan-outs in NON-widened read/paginate fns alone.
- DB-gated tests gate on `KOJI_DB_URL` (see the test-db setup memory): reconstruct `.env.test`, run `cargo run -p migration -- up`, copy the root `.env.test` into any worktree. A test that needs the DB must skip cleanly when `KOJI_DB_URL` is unset (follow the existing `*_db.rs` gate pattern).
- Write wire shape per import item is **exactly** `{ kind, name, geometry, mode?, parent?, projects[], route_parent?, on_collision }`. The import does NOT create koji *Property* associations from GeoJSON `properties` (scoped out of V1).
- Collision handling applies to **geofences** (by name). Routes always upsert by their existing `(name, mode, geofence)` identity.
- Run `cargo build`/`cargo test`/`cargo clippy` for `-p koji-db` and `-p koji` (the bin). Fix all red before advancing.

---

### Task A1: Make `geofence_project` + `property` writes generic over `C: ConnectionTrait`

**Files:**
- Modify: `crates/koji-db/src/db/geofence_project.rs`
- Modify: `crates/koji-db/src/db/property.rs`

**Interfaces:**
- Produces (new signatures; bodies unchanged — only the `db` param type widens):
  - `geofence_project::Query::upsert_related<C: ConnectionTrait>(db: &C, ids: &[serde_json::Value], fixed_val: u32, fixed_col: Column, other_col: Column, other_of: impl Fn(&Model) -> u32, make: impl Fn(u32) -> ActiveModel) -> Result<(), DbErr>`
  - `geofence_project::Query::upsert_related_by_geofence_id<C: ConnectionTrait>(db: &C, projects: &[serde_json::Value], geofence_id: u32) -> Result<(), DbErr>`
  - `property::Query::upsert<C: ConnectionTrait>(db: &C, id: u32, new_model: Json) -> Result<Model, ModelError>`
  - `property::Query::get_or_create_db_prop<C: ConnectionTrait>(db: &C, prop: &str) -> Result<Model, DbErr>`

These are leaf dependencies (lowest in the call tree) — widen them first so the next tasks' widenings compile.

- [ ] **Step 1: Confirm `ConnectionTrait` is in scope**

`crates/koji-db/src/db/geofence_project.rs` uses `use sea_orm::{InsertResult, entity::prelude::*};`. `sea_orm::entity::prelude::*` re-exports `ConnectionTrait`. `property.rs` uses `use sea_orm::entity::prelude::*;`. Both already have it. No import change needed (verify the build in Step 3).

- [ ] **Step 2: Widen the four signatures**

In `geofence_project.rs`, change the two fn signatures (bodies unchanged):

```rust
    async fn upsert_related<C: ConnectionTrait>(
        db: &C,
        ids: &[serde_json::Value],
        fixed_val: u32,
        fixed_col: Column,
        other_col: Column,
        other_of: impl Fn(&Model) -> u32,
        make: impl Fn(u32) -> ActiveModel,
    ) -> Result<(), DbErr> {
```

```rust
    pub async fn upsert_related_by_geofence_id<C: ConnectionTrait>(
        db: &C,
        projects: &[serde_json::Value],
        geofence_id: u32,
    ) -> Result<(), DbErr> {
```

In `property.rs`, change two signatures (bodies unchanged):

```rust
    pub async fn upsert<C: ConnectionTrait>(
        db: &C,
        id: u32,
        new_model: Json,
    ) -> Result<Model, ModelError> {
```

```rust
    pub async fn get_or_create_db_prop<C: ConnectionTrait>(
        db: &C,
        prop: &str,
    ) -> Result<Model, DbErr> {
```

Leave `property::Query::paginate`/`get_all`/`get_json_cache`/`upsert_json_return` and `geofence_project::Query::get_all`/`create`/`update`/`update_by_id`/`delete`/`upsert_related_by_project_id` on `&DatabaseConnection` (not in the import tx path).

- [ ] **Step 3: Build + run existing koji-db tests (no behavior change)**

Run: `cargo build -p koji-db 2>&1 | tail -20 && cargo test -p koji-db 2>&1 | tail -20`
Expected: builds clean; all existing koji-db tests still pass (pure signature widening — `&DatabaseConnection` still satisfies `&C`).

- [ ] **Step 4: Commit**

```bash
git add crates/koji-db/src/db/geofence_project.rs crates/koji-db/src/db/property.rs
git commit -m "refactor(db): widen geofence_project + property writes to generic C: ConnectionTrait"
```

---

### Task A2: Make `geofence_property` writes generic + serialize its concurrent upsert

**Files:**
- Modify: `crates/koji-db/src/db/geofence_property.rs`

**Interfaces:**
- Consumes: `property::Query::get_or_create_db_prop<C>` (A1).
- Produces:
  - `geofence_property::Query::upsert<C: ConnectionTrait>(db: &C, json: &Json, geofence_id: Option<u32>) -> Result<Model, ModelError>`
  - `geofence_property::Query::update_properties_by_geofence<C: ConnectionTrait>(db: &C, incoming: &[Json], geofence_id: Option<u32>) -> Result<Vec<Model>, ModelError>`
  - `geofence_property::Query::add_db_property<C: ConnectionTrait>(db: &C, id: u32, prop: &str) -> Result<Model, ModelError>`
  - `geofence_property::Query::update_values_for_property<C: ConnectionTrait>(db: &C, property_id: u32, new_value: &Option<String>) -> Result<UpdateResult, DbErr>`

- [ ] **Step 1: Widen the four signatures**

```rust
    pub async fn upsert<C: ConnectionTrait>(
        db: &C,
        json: &Json,
        geofence_id: Option<u32>,
    ) -> Result<Model, ModelError> {
```

```rust
    pub async fn add_db_property<C: ConnectionTrait>(
        db: &C,
        id: u32,
        prop: &str,
    ) -> Result<Model, ModelError> {
```

```rust
    pub async fn update_values_for_property<C: ConnectionTrait>(
        db: &C,
        property_id: u32,
        new_value: &Option<String>,
    ) -> Result<UpdateResult, DbErr> {
```

- [ ] **Step 2: Widen `update_properties_by_geofence` AND serialize its `try_join_all`**

Replace the whole fn body's concurrent map with a sequential loop. New fn:

```rust
    pub async fn update_properties_by_geofence<C: ConnectionTrait>(
        db: &C,
        incoming: &[Json],
        geofence_id: Option<u32>,
    ) -> Result<Vec<Model>, ModelError> {
        let mut existing = Entity::find()
            .filter(Column::GeofenceId.eq(geofence_id))
            .all(db)
            .await?
            .into_iter()
            .map(|model| (model.id, false))
            .collect::<HashMap<_, _>>();

        // Sequential (not try_join_all): may run inside a DatabaseTransaction,
        // where concurrent statements on the one connection are unsound.
        let mut models = Vec::with_capacity(incoming.len());
        for json in incoming {
            models.push(Query::upsert(db, json, geofence_id).await?);
        }

        for model in models.iter() {
            existing.entry(model.id).and_modify(|e| *e = true);
        }

        let existing: Vec<u32> = existing
            .into_iter()
            .filter_map(|(id, exists)| if !exists { Some(id) } else { None })
            .collect();

        if !existing.is_empty() {
            Entity::delete_many()
                .filter(Column::Id.is_in(existing))
                .exec(db)
                .await?;
        }
        Ok(models)
    }
```

- [ ] **Step 3: Drop the now-unused `futures::future` import**

`use futures::future;` (line ~8) is now unused (its only use was the deleted `try_join_all`). Remove that line. (If the build later reports it still used, restore — but `update_properties_by_geofence` was its only consumer in this file.)

- [ ] **Step 4: Build + test**

Run: `cargo build -p koji-db 2>&1 | tail -20 && cargo test -p koji-db 2>&1 | tail -20`
Expected: clean build (no unused-import warning), existing tests pass.

- [ ] **Step 5: Commit**

```bash
git add crates/koji-db/src/db/geofence_property.rs
git commit -m "refactor(db): geofence_property writes generic C + serialize concurrent upsert for tx-safety"
```

---

### Task A3: Make `geofence` writes + `get_one` generic + serialize its concurrent upsert

**Files:**
- Modify: `crates/koji-db/src/db/geofence/reads.rs` (`get_one` only)
- Modify: `crates/koji-db/src/db/geofence/writes.rs`

**Interfaces:**
- Consumes: `geofence_property::Query::{add_db_property,update_properties_by_geofence}<C>` (A2), `geofence_project::Query::upsert_related_by_geofence_id<C>` (A1), `property::Query::upsert<C>` (A1).
- Produces:
  - `geofence::Query::get_one<C: ConnectionTrait>(db: &C, id: String) -> Result<Model, ModelError>`
  - `geofence::Query::upsert<C: ConnectionTrait>(db: &C, id: u32, json: Json) -> Result<Model, ModelError>`
  - `geofence::Query::upsert_json_return<C: ConnectionTrait>(db: &C, id: u32, json: Json) -> Result<Json, ModelError>`
  - `geofence::Query::upsert_related_properties<C: ConnectionTrait>(db: &C, json: &serde_json::Value, geofence_id: u32) -> Result<(), ModelError>`
  - `geofence::Query::upsert_related_projects<C: ConnectionTrait>(db: &C, json: &serde_json::Value, geofence_id: u32) -> Result<(), DbErr>`
  - `geofence::Query::update_related_route_names<C: ConnectionTrait>(conn: &C, old_model: &Model, new_name: String) -> Result<UpdateResult, DbErr>`

- [ ] **Step 1: Widen `get_one` in reads.rs**

`crates/koji-db/src/db/geofence/reads.rs:62` — change only the signature (body unchanged; `Entity::find_by_id`/`find().filter` are `ConnectionTrait`-generic):

```rust
    pub async fn get_one<C: ConnectionTrait>(db: &C, id: String) -> Result<Model, ModelError> {
```

Confirm `ConnectionTrait` is in scope in reads.rs (the geofence module's `use super::*;` pulls in the entity prelude that re-exports it; the build in Step 5 confirms).

- [ ] **Step 2: Widen the four non-`upsert` writes in writes.rs**

```rust
    pub async fn update_related_route_names<C: ConnectionTrait>(
        conn: &C,
        old_model: &Model,
        new_name: String,
    ) -> Result<UpdateResult, DbErr> {
```

```rust
    pub async fn upsert_related_projects<C: ConnectionTrait>(
        db: &C,
        json: &serde_json::Value,
        geofence_id: u32,
    ) -> Result<(), DbErr> {
```

```rust
    pub async fn upsert_json_return<C: ConnectionTrait>(
        db: &C,
        id: u32,
        json: Json,
    ) -> Result<Json, ModelError> {
```

- [ ] **Step 3: Widen `upsert_related_properties` AND serialize its `try_join_all`**

Replace the concurrent `future::try_join_all` block with a sequential loop. New fn:

```rust
    pub async fn upsert_related_properties<C: ConnectionTrait>(
        db: &C,
        json: &serde_json::Value,
        geofence_id: u32,
    ) -> Result<(), ModelError> {
        if let Some(properties) = json.get("properties")
            && let Some(properties) = properties.as_array()
        {
            let mut existing = vec![];
            let mut new_props = vec![];
            properties.iter().for_each(|property| {
                if let Some(prop_map) = property.as_object() {
                    if prop_map.contains_key("property_id") {
                        existing.push(property.clone())
                    } else {
                        new_props.push(property.clone())
                    }
                }
            });

            // Sequential (not try_join_all): may run inside a DatabaseTransaction.
            let mut upserted_props = Vec::with_capacity(new_props.len());
            for result in new_props.clone() {
                upserted_props.push(property::Query::upsert(db, 0, result).await?);
            }

            upserted_props
                .into_iter()
                .enumerate()
                .for_each(|(i, prop)| {
                    existing.push(json!({
                        "value": new_props[i]["value"],
                        "property_id": prop.id,
                        "geofence_id": geofence_id,
                    }))
                });

            geofence_property::Query::update_properties_by_geofence(
                db,
                &existing,
                Some(geofence_id),
            )
            .await?;
        };
        Ok(())
    }
```

- [ ] **Step 4: Widen `upsert` (body otherwise unchanged)**

```rust
    pub async fn upsert<C: ConnectionTrait>(db: &C, id: u32, json: Json) -> Result<Model, ModelError> {
```

The body already calls only widened fns (`Query::get_one`, `update_related_route_names`, `geofence_property::Query::add_db_property`, `upsert_related_projects`, `upsert_related_properties`) plus `ConnectionTrait`-generic entity ops (`Entity::find_by_id`, `new_model.insert/update`). Leave `upsert_from_geometry`/`upsert_koji_item`/`associate_parent`/`assign`/`delete` on `&DatabaseConnection` (not in the import tx path — their `future::try_join_all` at line ~186 stays, so keep the `use ... future` import).

- [ ] **Step 5: Build + test**

Run: `cargo build -p koji-db 2>&1 | tail -20 && cargo test -p koji-db 2>&1 | tail -20`
Expected: clean; existing tests pass.

- [ ] **Step 6: Commit**

```bash
git add crates/koji-db/src/db/geofence/reads.rs crates/koji-db/src/db/geofence/writes.rs
git commit -m "refactor(db): geofence writes + get_one generic C + serialize concurrent prop upsert"
```

---

### Task A4: Make `route` upsert generic over `C: ConnectionTrait`

**Files:**
- Modify: `crates/koji-db/src/db/route.rs`

**Interfaces:**
- Produces:
  - `route::Query::upsert<C: ConnectionTrait>(db: &C, id: u32, json: Json) -> Result<Model, ModelError>`
  - `route::Query::upsert_json_return<C: ConnectionTrait>(db: &C, id: u32, json: Json) -> Result<Json, ModelError>`

- [ ] **Step 1: Widen the two signatures (bodies unchanged)**

```rust
    pub async fn upsert<C: ConnectionTrait>(db: &C, id: u32, json: Json) -> Result<Model, ModelError> {
```

```rust
    pub async fn upsert_json_return<C: ConnectionTrait>(
        db: &C,
        id: u32,
        json: Json,
    ) -> Result<Json, ModelError> {
```

Leave `create`/`update`/`upsert_from_geometry`/`resolve_geofence_id`/reads on `&DatabaseConnection` (the import builds route rows via `upsert` with an explicit `geofence_id` in the JSON — see A5).

`ConnectionTrait` is in scope via `use sea_orm::{... entity::prelude::*}` (line 8 imports `entity::prelude::*`). Build confirms.

- [ ] **Step 2: Build + test**

Run: `cargo build -p koji-db 2>&1 | tail -20 && cargo test -p koji-db 2>&1 | tail -20`
Expected: clean; existing tests (incl. the route `to_koji_tests` module) pass.

- [ ] **Step 3: Commit**

```bash
git add crates/koji-db/src/db/route.rs
git commit -m "refactor(db): route upsert generic C: ConnectionTrait"
```

---

### Task A5: koji-db `import` module — dry-run validation + transactional commit

**Files:**
- Create: `crates/koji-db/src/db/import.rs`
- Modify: `crates/koji-db/src/db/mod.rs` (add `pub mod import;`)
- Test: inline `#[cfg(test)] mod tests` in `import.rs` (pure-validation tests, no DB) + a DB-gated `crates/koji-db/tests/import_db.rs` (integration).

**Interfaces:**
- Consumes: `geofence::Query::{get_one,upsert}<C>`, `route::Query::upsert<C>`, `geofence::Entity` (name lookup).
- Produces (HTTP-agnostic types the service maps onto):

```rust
pub enum ImportKind { Geofence, Route }
pub enum OnCollision { Skip, Overwrite }
pub enum ImportAction { Create, Update, Skip, Fail }

pub struct ImportItem {
    pub kind: ImportKind,
    pub name: String,
    pub geometry: serde_json::Value,     // GeoJSON geometry (already normalized by /convert)
    pub mode: Option<String>,
    pub parent: Option<String>,          // geofence parent BY NAME
    pub projects: Vec<u32>,
    pub route_parent: Option<String>,    // routes: parent geofence BY NAME
    pub on_collision: OnCollision,
}

pub struct ImportOutcome {
    pub index: usize,
    pub name: String,
    pub action: ImportAction,
    pub id: Option<u32>,
    pub reason: Option<String>,
}

pub struct ImportSummary { pub create: usize, pub update: usize, pub skip: usize, pub fail: usize }

pub struct ImportResult {
    pub committed: bool,
    pub summary: ImportSummary,
    pub results: Vec<ImportOutcome>,
}

pub async fn import(db: &DatabaseConnection, items: Vec<ImportItem>, dry_run: bool) -> Result<ImportResult, ModelError>
```

Design notes for the implementer:
- **Validation (runs for BOTH dry-run and commit, before any write):** per item — name non-empty (trimmed); no duplicate names *within* the geofence items of the batch; geometry is a JSON object with a `type`; `parent`/`route_parent` resolvable against (existing geofence names) ∪ (batch geofence names). Any failure → that item's predicted `ImportAction::Fail` with a `reason`.
- **Collision detection:** a geofence item whose name matches an existing geofence row → predicted `Update` if `on_collision==Overwrite`, else `Skip`. (Query existing geofences by name once up-front.)
- **Dry-run:** return the predicted outcomes with `committed:false`. No txn, no writes.
- **Commit:** if validation produced ANY `Fail`, return `committed:false` with those outcomes and DO NOT open a txn. Otherwise `let txn = db.begin().await?;` then: upsert all geofence items first (skip the `Skip` ones), recording `name -> new id` in a map; resolve each route's `route_parent` name → id (batch map ∪ existing); upsert routes (build the route JSON with the resolved `geofence_id`, `name`, `mode`, `geometry`); on any error `txn.rollback().await?` and return `committed:false` + a `Fail` outcome for the offending item; on success `txn.commit().await?` and return `committed:true`.
- `db.begin()` requires `use sea_orm::TransactionTrait;`. The `&txn` (a `&DatabaseTransaction`) satisfies the generic `&C: ConnectionTrait` widened in A1–A4.
- Geofence upsert JSON shape (matches `to_geofence`): `{ "name", "mode"?, "geometry", "parent"? (id), "projects": [..], "properties": [] }`. Resolve a `parent` NAME to its id from the batch map ∪ existing before building this JSON. Use `geofence::Query::upsert(&txn, 0, json)`.
- Route upsert JSON shape (matches `to_route`): `{ "name", "mode"?, "geometry", "geofence_id" }`. Use `route::Query::upsert(&txn, 0, json)`.

- [ ] **Step 1: Write the failing pure-validation tests**

In `import.rs`'s `#[cfg(test)] mod tests` (no DB — exercises only the validation helper, see Step 3 which factors validation into a pure `fn validate(items, existing_names) -> Vec<Option<String>>` returning a per-item failure reason or None):

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn item(kind: ImportKind, name: &str) -> ImportItem {
        ImportItem {
            kind,
            name: name.to_string(),
            geometry: serde_json::json!({ "type": "Polygon", "coordinates": [] }),
            mode: None,
            parent: None,
            projects: vec![],
            route_parent: None,
            on_collision: OnCollision::Skip,
        }
    }

    #[test]
    fn empty_name_is_a_failure() {
        let items = vec![item(ImportKind::Geofence, "  ")];
        let reasons = validate(&items, &HashSet::new());
        assert!(reasons[0].as_deref().unwrap().contains("name"));
    }

    #[test]
    fn duplicate_geofence_names_in_batch_fail_both() {
        let items = vec![item(ImportKind::Geofence, "Dup"), item(ImportKind::Geofence, "Dup")];
        let reasons = validate(&items, &HashSet::new());
        assert!(reasons[0].is_some() && reasons[1].is_some());
    }

    #[test]
    fn route_parent_unresolvable_fails() {
        let mut r = item(ImportKind::Route, "patrol");
        r.geometry = serde_json::json!({ "type": "MultiPoint", "coordinates": [] });
        r.route_parent = Some("NoSuchFence".to_string());
        let reasons = validate(&[r], &HashSet::new());
        assert!(reasons[0].as_deref().unwrap().contains("parent"));
    }

    #[test]
    fn route_parent_resolvable_against_batch_geofence_passes() {
        let fence = item(ImportKind::Geofence, "Region-A");
        let mut route = item(ImportKind::Route, "patrol");
        route.geometry = serde_json::json!({ "type": "MultiPoint", "coordinates": [] });
        route.route_parent = Some("Region-A".to_string());
        let reasons = validate(&[fence, route], &HashSet::new());
        assert!(reasons[1].is_none());
    }

    #[test]
    fn missing_geometry_type_fails() {
        let mut g = item(ImportKind::Geofence, "NoGeom");
        g.geometry = serde_json::json!({ "coordinates": [] });
        let reasons = validate(&[g], &HashSet::new());
        assert!(reasons[0].as_deref().unwrap().contains("geometry"));
    }
}
```

- [ ] **Step 2: Run the tests, verify they fail**

Run: `cargo test -p koji-db import::tests 2>&1 | tail -20`
Expected: FAIL — `import` module / `validate` not found.

- [ ] **Step 3: Implement the module**

Create `crates/koji-db/src/db/import.rs`. Factor validation into the pure `validate` fn the tests call, then the `import` orchestrator. Full implementation:

```rust
//! Atomic bulk import: validate a batch (dry-run) then commit it in one
//! transaction. Geofences upsert first (collecting name->id) so routes resolve
//! their parent fence in the same tx; any hard failure rolls the whole tx back.

use std::collections::{HashMap, HashSet};

use sea_orm::{DatabaseConnection, EntityTrait, TransactionTrait};
use serde_json::json;

use crate::db::{geofence, route};
use crate::error::ModelError;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImportKind { Geofence, Route }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OnCollision { Skip, Overwrite }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImportAction { Create, Update, Skip, Fail }

#[derive(Clone, Debug)]
pub struct ImportItem {
    pub kind: ImportKind,
    pub name: String,
    pub geometry: serde_json::Value,
    pub mode: Option<String>,
    pub parent: Option<String>,
    pub projects: Vec<u32>,
    pub route_parent: Option<String>,
    pub on_collision: OnCollision,
}

#[derive(Clone, Debug)]
pub struct ImportOutcome {
    pub index: usize,
    pub name: String,
    pub action: ImportAction,
    pub id: Option<u32>,
    pub reason: Option<String>,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ImportSummary { pub create: usize, pub update: usize, pub skip: usize, pub fail: usize }

#[derive(Clone, Debug)]
pub struct ImportResult {
    pub committed: bool,
    pub summary: ImportSummary,
    pub results: Vec<ImportOutcome>,
}

/// Pure per-item validation. Returns `Some(reason)` for each item that cannot
/// be imported, else `None`. `existing_names` = geofence names already in the DB.
pub(crate) fn validate(items: &[ImportItem], existing_names: &HashSet<String>) -> Vec<Option<String>> {
    // Names of geofences that WILL exist after this batch (existing ∪ batch).
    let mut batch_fences: HashSet<String> = HashSet::new();
    let mut seen_fences: HashSet<String> = HashSet::new();
    let mut dup_fences: HashSet<String> = HashSet::new();
    for it in items {
        if it.kind == ImportKind::Geofence {
            let n = it.name.trim().to_string();
            if !n.is_empty() {
                if !seen_fences.insert(n.clone()) {
                    dup_fences.insert(n.clone());
                }
                batch_fences.insert(n);
            }
        }
    }
    let resolvable = |name: &str| existing_names.contains(name) || batch_fences.contains(name);

    items
        .iter()
        .map(|it| {
            let name = it.name.trim();
            if name.is_empty() {
                return Some("empty name".to_string());
            }
            if it.geometry.get("type").and_then(|t| t.as_str()).is_none() {
                return Some("geometry missing a `type`".to_string());
            }
            if it.kind == ImportKind::Geofence && dup_fences.contains(name) {
                return Some(format!("duplicate geofence name `{name}` in batch"));
            }
            if let Some(p) = it.parent.as_deref() {
                if !resolvable(p) {
                    return Some(format!("parent `{p}` not found"));
                }
            }
            if it.kind == ImportKind::Route {
                match it.route_parent.as_deref() {
                    Some(p) if !resolvable(p) => return Some(format!("route parent `{p}` not found")),
                    None => return Some("route has no parent geofence".to_string()),
                    _ => {}
                }
            }
            None
        })
        .collect()
}

/// Load the set of existing geofence names (for collision + parent resolution).
async fn existing_geofence_names(db: &DatabaseConnection) -> Result<HashSet<String>, ModelError> {
    let rows = geofence::Entity::find().all(db).await?;
    Ok(rows.into_iter().map(|m| m.name).collect())
}

/// Map existing geofence name -> id (for parent resolution against the DB).
async fn existing_geofence_ids(db: &DatabaseConnection) -> Result<HashMap<String, u32>, ModelError> {
    let rows = geofence::Entity::find().all(db).await?;
    Ok(rows.into_iter().map(|m| (m.name, m.id)).collect())
}

pub async fn import(
    db: &DatabaseConnection,
    items: Vec<ImportItem>,
    dry_run: bool,
) -> Result<ImportResult, ModelError> {
    let existing_names = existing_geofence_names(db).await?;
    let reasons = validate(&items, &existing_names);

    // Predict actions for every item (used by both dry-run and the fail-fast guard).
    let predicted: Vec<ImportOutcome> = items
        .iter()
        .enumerate()
        .map(|(index, it)| {
            let name = it.name.trim().to_string();
            if let Some(reason) = reasons[index].clone() {
                return ImportOutcome { index, name, action: ImportAction::Fail, id: None, reason: Some(reason) };
            }
            if it.kind == ImportKind::Geofence && existing_names.contains(&name) {
                let action = match it.on_collision {
                    OnCollision::Overwrite => ImportAction::Update,
                    OnCollision::Skip => ImportAction::Skip,
                };
                let reason = (action == ImportAction::Skip)
                    .then(|| "collision, on_collision=skip".to_string());
                return ImportOutcome { index, name, action, id: None, reason };
            }
            ImportOutcome { index, name, action: ImportAction::Create, id: None, reason: None }
        })
        .collect();

    let has_fail = predicted.iter().any(|o| o.action == ImportAction::Fail);

    if dry_run || has_fail {
        return Ok(ImportResult {
            committed: false,
            summary: summarize(&predicted),
            results: predicted,
        });
    }

    // --- Commit pass: one transaction. ---
    let mut name_to_id = existing_geofence_ids(db).await?;
    let txn = db.begin().await?;
    let mut results = Vec::with_capacity(items.len());

    // Geofences first (so routes resolve parents in the same tx).
    for (index, it) in items.iter().enumerate() {
        if it.kind != ImportKind::Geofence {
            continue;
        }
        let name = it.name.trim().to_string();
        let predicted_action = predicted[index].action;
        if predicted_action == ImportAction::Skip {
            results.push(predicted[index].clone());
            continue;
        }
        let parent_id = it.parent.as_deref().and_then(|p| name_to_id.get(p).copied());
        let mut body = json!({
            "name": name,
            "geometry": it.geometry,
            "projects": it.projects,
            "properties": [],
        });
        if let Some(mode) = &it.mode {
            body["mode"] = json!(mode);
        }
        if let Some(pid) = parent_id {
            body["parent"] = json!(pid);
        }
        match geofence::Query::upsert(&txn, 0, body).await {
            Ok(model) => {
                name_to_id.insert(name.clone(), model.id);
                results.push(ImportOutcome {
                    index,
                    name,
                    action: predicted_action,
                    id: Some(model.id),
                    reason: None,
                });
            }
            Err(e) => {
                txn.rollback().await?;
                return Ok(fail_result(index, name, e.to_string()));
            }
        }
    }

    // Routes second.
    for (index, it) in items.iter().enumerate() {
        if it.kind != ImportKind::Route {
            continue;
        }
        let name = it.name.trim().to_string();
        let parent_id = it.route_parent.as_deref().and_then(|p| name_to_id.get(p).copied());
        let Some(parent_id) = parent_id else {
            txn.rollback().await?;
            return Ok(fail_result(index, name, "route parent not found".to_string()));
        };
        let mut body = json!({
            "name": name,
            "geometry": it.geometry,
            "geofence_id": parent_id,
        });
        if let Some(mode) = &it.mode {
            body["mode"] = json!(mode);
        }
        match route::Query::upsert(&txn, 0, body).await {
            Ok(model) => results.push(ImportOutcome {
                index,
                name,
                action: ImportAction::Create,
                id: Some(model.id),
                reason: None,
            }),
            Err(e) => {
                txn.rollback().await?;
                return Ok(fail_result(index, name, e.to_string()));
            }
        }
    }

    txn.commit().await?;
    Ok(ImportResult { committed: true, summary: summarize(&results), results })
}

fn summarize(results: &[ImportOutcome]) -> ImportSummary {
    let mut s = ImportSummary::default();
    for o in results {
        match o.action {
            ImportAction::Create => s.create += 1,
            ImportAction::Update => s.update += 1,
            ImportAction::Skip => s.skip += 1,
            ImportAction::Fail => s.fail += 1,
        }
    }
    s
}

/// Build a rolled-back result whose only outcome is the failure that aborted
/// the transaction. `committed:false` signals nothing was written.
fn fail_result(index: usize, name: String, reason: String) -> ImportResult {
    let results = vec![ImportOutcome {
        index,
        name,
        action: ImportAction::Fail,
        id: None,
        reason: Some(reason),
    }];
    ImportResult { committed: false, summary: summarize(&results), results }
}
```

Add to `crates/koji-db/src/db/mod.rs`: `pub mod import;` (alongside the other `pub mod`s).

- [ ] **Step 4: Run the pure tests, verify green**

Run: `cargo test -p koji-db import::tests 2>&1 | tail -20`
Expected: PASS (5 cases).

- [ ] **Step 5: Write the DB-gated integration test**

Create `crates/koji-db/tests/import_db.rs`. Follow the existing `*_db.rs` gate pattern (skip when `KOJI_DB_URL` unset). Cover: create-then-idempotent-retry (no dups), collision skip vs overwrite, route parent resolved from a batch fence, rollback on a bad route parent leaves nothing written.

```rust
//! DB-gated integration for the atomic import. Gated on `KOJI_DB_URL`.
#![cfg(test)]

use std::collections::HashSet;
use koji_db::db::import::{import, ImportItem, ImportKind, OnCollision, ImportAction};
use sea_orm::{Database, DatabaseConnection};

async fn conn() -> Option<DatabaseConnection> {
    let url = std::env::var("KOJI_DB_URL").ok()?;
    Database::connect(url).await.ok()
}

fn fence(name: &str) -> ImportItem {
    ImportItem {
        kind: ImportKind::Geofence,
        name: name.to_string(),
        geometry: serde_json::json!({ "type": "Polygon", "coordinates": [[[0.0,0.0],[1.0,0.0],[1.0,1.0],[0.0,0.0]]] }),
        mode: Some("pokemon".to_string()),
        parent: None,
        projects: vec![],
        route_parent: None,
        on_collision: OnCollision::Skip,
    }
}

#[tokio::test]
async fn import_creates_then_retry_is_idempotent() {
    let Some(db) = conn().await else { eprintln!("skip: KOJI_DB_URL unset"); return; };
    let name = "itest-fence-idem";
    let r1 = import(&db, vec![fence(name)], false).await.unwrap();
    assert!(r1.committed);
    assert_eq!(r1.summary.create + r1.summary.update + r1.summary.skip, 1);

    // Retry: name now collides; default on_collision=skip → reported skip, no dup.
    let r2 = import(&db, vec![fence(name)], false).await.unwrap();
    assert!(r2.committed);
    assert_eq!(r2.results[0].action, ImportAction::Skip);
}

#[tokio::test]
async fn dry_run_writes_nothing_and_predicts() {
    let Some(db) = conn().await else { eprintln!("skip: KOJI_DB_URL unset"); return; };
    let r = import(&db, vec![fence("itest-dryrun-only")], true).await.unwrap();
    assert!(!r.committed);
    assert_eq!(r.results[0].action, ImportAction::Create);
}

#[tokio::test]
async fn rollback_on_unresolvable_route_parent_leaves_nothing() {
    let Some(db) = conn().await else { eprintln!("skip: KOJI_DB_URL unset"); return; };
    // A valid fence + a route whose parent does not exist → validation fails the
    // route at dry-run, so commit is refused (committed:false), fence NOT written.
    let mut route = fence("itest-orphan-route");
    route.kind = ImportKind::Route;
    route.geometry = serde_json::json!({ "type": "MultiPoint", "coordinates": [[0.0,0.0]] });
    route.route_parent = Some("itest-nonexistent-parent".to_string());
    let r = import(&db, vec![fence("itest-rollback-fence"), route], false).await.unwrap();
    assert!(!r.committed);
    assert!(r.results.iter().any(|o| o.action == ImportAction::Fail));
}
```

- [ ] **Step 6: Run the integration test (DB up)**

Run: `cargo test -p koji-db --test import_db 2>&1 | tail -30`
Expected: PASS if `KOJI_DB_URL` is set (3 cases); cleanly skipped otherwise. If the DB is available, ensure it is migrated (`cargo run -p migration -- up`).

- [ ] **Step 7: Clippy + commit**

Run: `cargo clippy -p koji-db 2>&1 | tail -20`
Expected: 0 warnings.

```bash
git add crates/koji-db/src/db/import.rs crates/koji-db/src/db/mod.rs crates/koji-db/tests/import_db.rs
git commit -m "feat(db): atomic import module — dry-run validation + transactional commit"
```

---

### Task A6: Service `/import` handler + DTOs + dry-run mapping

**Files:**
- Create: `crates/koji-service/src/public/v2/import.rs`
- Modify: `crates/koji-service/src/public/v2/mod.rs` (add `pub(crate) mod import;`)
- Test: inline `#[cfg(test)] mod tests` in `import.rs` (pure DTO→koji-db mapping; no DB).

**Interfaces:**
- Consumes: `koji_db::db::import::{import, ImportItem, ImportKind, OnCollision, ImportResult, ImportAction}`.
- Produces: `pub(crate) async fn import_handler(conn: web::Data<KojiDb>, body: web::Json<ImportRequest>) -> Result<HttpResponse, ServiceError>` + `pub(crate) fn to_db_items(req: &ImportRequest) -> Vec<koji_db::db::import::ImportItem>` (the mapping, unit-tested).

- [ ] **Step 1: Write the failing mapping test**

In `import.rs`'s test module:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_request_items_to_db_items_with_defaults() {
        let req: ImportRequest = serde_json::from_value(serde_json::json!({
            "dry_run": true,
            "items": [
                { "kind": "geofence", "name": "A", "geometry": { "type": "Polygon", "coordinates": [] } },
                { "kind": "route", "name": "r", "geometry": { "type": "MultiPoint", "coordinates": [] },
                  "route_parent": "A", "on_collision": "overwrite", "projects": [1, 2] }
            ]
        })).unwrap();
        let items = to_db_items(&req);
        assert_eq!(items.len(), 2);
        // defaults: missing on_collision -> Skip, missing projects -> empty
        assert!(matches!(items[0].kind, koji_db::db::import::ImportKind::Geofence));
        assert!(matches!(items[0].on_collision, koji_db::db::import::OnCollision::Skip));
        assert!(items[0].projects.is_empty());
        assert!(matches!(items[1].kind, koji_db::db::import::ImportKind::Route));
        assert!(matches!(items[1].on_collision, koji_db::db::import::OnCollision::Overwrite));
        assert_eq!(items[1].route_parent.as_deref(), Some("A"));
        assert_eq!(items[1].projects, vec![1, 2]);
    }
}
```

- [ ] **Step 2: Run it, verify it fails**

Run: `cargo test -p koji import::tests 2>&1 | tail -20`
Expected: FAIL — module/`to_db_items` not found.

- [ ] **Step 3: Implement the handler + DTOs + mapping**

Create `crates/koji-service/src/public/v2/import.rs`:

```rust
//! `POST /internal/import` — atomic bulk import. Maps the HTTP DTO to the
//! koji-db `import` orchestrator (one tx; dry-run = validate-only preview).

use actix_web::{HttpResponse, web};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use koji_db::KojiDb;
use koji_db::db::import as dbimport;

use crate::utils::api_response::ApiResponse;
use crate::utils::error::ServiceError;

#[derive(Debug, Clone, Copy, Deserialize, Serialize, ToSchema, Default)]
#[serde(rename_all = "lowercase")]
pub(crate) enum ItemKind { #[default] Geofence, Route }

#[derive(Debug, Clone, Copy, Deserialize, Serialize, ToSchema, Default)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Collision { #[default] Skip, Overwrite }

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub(crate) struct ImportItemDto {
    #[serde(default)]
    pub kind: ItemKind,
    pub name: String,
    #[schema(value_type = Object)]
    pub geometry: serde_json::Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
    #[serde(default)]
    pub projects: Vec<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub route_parent: Option<String>,
    #[serde(default)]
    pub on_collision: Collision,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub(crate) struct ImportRequest {
    #[serde(default)]
    pub dry_run: bool,
    pub items: Vec<ImportItemDto>,
}

pub(crate) fn to_db_items(req: &ImportRequest) -> Vec<dbimport::ImportItem> {
    req.items
        .iter()
        .map(|it| dbimport::ImportItem {
            kind: match it.kind {
                ItemKind::Geofence => dbimport::ImportKind::Geofence,
                ItemKind::Route => dbimport::ImportKind::Route,
            },
            name: it.name.clone(),
            geometry: it.geometry.clone(),
            mode: it.mode.clone(),
            parent: it.parent.clone(),
            projects: it.projects.clone(),
            route_parent: it.route_parent.clone(),
            on_collision: match it.on_collision {
                Collision::Skip => dbimport::OnCollision::Skip,
                Collision::Overwrite => dbimport::OnCollision::Overwrite,
            },
        })
        .collect()
}

fn result_to_json(r: &dbimport::ImportResult) -> serde_json::Value {
    let action = |a: dbimport::ImportAction| match a {
        dbimport::ImportAction::Create => "create",
        dbimport::ImportAction::Update => "update",
        dbimport::ImportAction::Skip => "skip",
        dbimport::ImportAction::Fail => "fail",
    };
    serde_json::json!({
        "committed": r.committed,
        "summary": {
            "create": r.summary.create, "update": r.summary.update,
            "skip": r.summary.skip, "fail": r.summary.fail,
        },
        "results": r.results.iter().map(|o| serde_json::json!({
            "index": o.index, "name": o.name, "action": action(o.action),
            "id": o.id, "reason": o.reason,
        })).collect::<Vec<_>>(),
    })
}

/// `POST /internal/import`
pub(crate) async fn import_handler(
    conn: web::Data<KojiDb>,
    body: web::Json<ImportRequest>,
) -> Result<HttpResponse, ServiceError> {
    let req = body.into_inner();
    let dry_run = req.dry_run;
    let items = to_db_items(&req);
    let result = dbimport::import(&conn.koji, items, dry_run)
        .await
        .map_err(ServiceError::internal)?;
    Ok(HttpResponse::Ok().json(ApiResponse::Ok {
        data: result_to_json(&result),
        meta: None,
    }))
}
```

Add `pub(crate) mod import;` to `crates/koji-service/src/public/v2/mod.rs`. (Confirm `ServiceError::internal` exists — `geofences.rs` uses `ServiceError::internal` at create; reuse it. Confirm the `ApiResponse::Ok { data, meta }` shape matches `geofences.rs` usage.)

- [ ] **Step 4: Run the mapping test, verify green**

Run: `cargo test -p koji import::tests 2>&1 | tail -20`
Expected: PASS.

- [ ] **Step 5: Build the bin + clippy**

Run: `cargo build -p koji 2>&1 | tail -10 && cargo clippy -p koji 2>&1 | tail -20`
Expected: clean.

- [ ] **Step 6: Commit**

```bash
git add crates/koji-service/src/public/v2/import.rs crates/koji-service/src/public/v2/mod.rs
git commit -m "feat(service): POST /internal/import handler + DTOs (maps to koji-db import)"
```

---

### Task A7: Wire `/internal/import` + `/internal/geometry` alias into the internal scope

**Files:**
- Modify: `crates/koji-service/src/internal/mod.rs`

**Interfaces:**
- Consumes: `import::import_handler` (A6), the existing `v2::geometry` scope/handlers.
- Produces: `POST /internal/import` and `POST /internal/geometry/convert` (+ siblings) routes.

- [ ] **Step 1: Confirm the geometry registration form (already verified)**

`crates/koji-service/src/public/v2/geometry.rs:190` defines `pub(crate) fn scope() -> actix_web::Scope` which itself wraps `web::scope("/geometry")` with `#[post("/convert")]` / `#[post("/simplify")]` / `#[post("/merge-points")]` / `#[post("/area")]` handlers. `lib.rs` already mounts it under `/api/v2` via `.service(public::v2::geometry::scope())` (lib.rs:148). So aliasing under `/internal` is the identical one-liner — `.service(v2::geometry::scope())` mounts `/internal/geometry/convert` etc. No path wrapping needed.

- [ ] **Step 2: Add both services to the internal `scope()`**

In `crates/koji-service/src/internal/mod.rs`, inside `scope()`, after the existing `.service(...)` chain (e.g. after the realtime resource at the end), add:

```rust
        // Atomic bulk import (one tx; dry-run preview).
        .service(
            web::resource("/import")
                .route(web::post().to(v2::import::import_handler)),
        )
        // Geometry normalize/convert (+ simplify/merge-points/area), aliased from
        // /api/v2 so the admin client (which only talks to /internal) can reach it.
        .service(v2::geometry::scope())
```

- [ ] **Step 3: Build the bin**

Run: `cargo build -p koji 2>&1 | tail -10`
Expected: clean.

- [ ] **Step 4: Write a routing smoke test (auth + method)**

Add to `crates/koji-service` an actix test (follow an existing handler test in the crate for the `test::init_service` + app-factory pattern; if none, place it inline in `import.rs` behind `#[cfg(test)]` using `actix_web::test`). Assert that `POST /internal/import` is routed (a request without the session bearer returns 401 from the middleware, NOT 404 — proving the route exists and is auth-gated). If wiring an isolated `App` with the real middleware is heavy, instead assert routing against a minimal `App::new().service(crate::internal::scope())` and check a `POST /internal/import` with an empty body reaches the handler (400/500 for bad body, not 404).

```rust
#[actix_web::test]
async fn internal_import_route_is_registered() {
    use actix_web::{test, App};
    let app = test::init_service(
        App::new().service(crate::internal::scope())
    ).await;
    // No auth middleware here → handler is reached; empty/invalid body => 400, not 404.
    let req = test::TestRequest::post().uri("/internal/import").set_json(serde_json::json!({})).to_request();
    let resp = test::call_service(&app, req).await;
    assert_ne!(resp.status().as_u16(), 404, "POST /internal/import must be routed");
}
```

(`web::Json<ImportRequest>` with a `{}` body fails to deserialize `items` → 400, which still proves routing. If `scope()` requires app data the test must `.app_data(...)` the `KojiDb` — if that is heavy, skip this DB-coupled assertion and keep only the 404-negative check by registering just the `/import` resource.)

- [ ] **Step 5: Run the routing test**

Run: `cargo test -p koji internal_import_route 2>&1 | tail -20`
Expected: PASS (status ≠ 404).

- [ ] **Step 6: Commit**

```bash
git add crates/koji-service/src/internal/mod.rs crates/koji-service/src/public/v2/import.rs
git commit -m "feat(internal): wire POST /internal/import + /internal/geometry alias"
```

---

## Final Verification (controller, after all tasks)

- [ ] `cargo build -p koji-db -p koji 2>&1 | tail` — clean.
- [ ] `cargo clippy -p koji-db -p koji 2>&1 | tail` — 0 warnings.
- [ ] `cargo test -p koji-db -p koji 2>&1 | tail` — all green (DB-gated import tests pass with `KOJI_DB_URL` set + migrated, else skip cleanly).
- [ ] Restart `koji-server` (env vars set inline at launch per the import-wizard-slice memory) and live-probe through the running server with a session cookie:
  - `POST /internal/import {dry_run:true, items:[one geofence]}` → `{committed:false, summary:{create:1}, results:[{action:"create"}]}`.
  - `POST /internal/import {dry_run:false, ...}` → `{committed:true}`; re-POST the same → `results:[{action:"skip"}]` (idempotent, no dup).
  - `POST /internal/geometry/convert {area:<geojson FC>, output:{...}}` → 200 normalized FeatureCollection (proves the alias).
  - Confirm all three require the session bearer (401 without).

**Next:** Phase B (frontend core) — `/import` route, `<Stepper>`, Source(GeoJSON)/Map&Name/Assign/Review&Commit, `dataProvider.import`, guard-on-leave.
