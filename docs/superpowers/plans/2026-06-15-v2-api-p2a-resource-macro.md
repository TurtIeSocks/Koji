# v2 API — Phase 2a: `koji_resource!` macro + plain CRUD resources — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:subagent-driven-development / executing-plans + superpowers:test-driven-development. Steps use `- [ ]` checkboxes.

**Goal:** A `koji_resource!` proc-macro (in the `macros` crate) that generates typed Create/Patch DTOs + the five REST handlers + a `scope()` for a plain-JSON resource, wired to `ServiceError` (404), `201`+`Location`, `204`, the v2 envelope, and `?page=&per_page=` pagination. Apply it to **projects / properties / tile-servers**, replacing the existing `crud_resource!`. Standardize v2 pagination on `?page=&per_page=` (retrofit `/jobs`).

**Architecture:** The macro is *function-like* (`koji_resource! { ... }`), invoked in `koji-service`, emitting a resource submodule. It does NOT introspect the sea-orm `Model` (cross-crate); the caller passes the Create field list explicitly. Generated code references koji-service types by `crate::…` path (resolved at the call site), exactly like the existing `crud_query`/`fort_query` macros reference `crate::…`/`koji_core::…`. The Patch DTO is the Create DTO with every field `Option`. koji-db's existing `Query::paginate` provides the page/total data.

**Tech Stack:** Rust, `syn` 2 / `quote` (already in `macros`), actix-web, sea-orm, serde.

## Delegate-mode decisions (recorded for review)

- **Pagination wire = `?page=&per_page=`** (NOT spec A10's `?limit/&offset`). Reason: koji-db's existing `Query::paginate(AdminReqParsed)` is page-based, returns `total`/`has_next`/`has_prev`, and `Meta` is already page-shaped — 1:1 reuse, zero new query code. Task 1 retrofits P0's `Pagination` + P1's `/jobs` list so the whole v2 surface is consistent.
- **Macro is implementer-written from this contract** (TDD, compiler-in-the-loop) rather than pasted here. The contract below is exact; deviations must be flagged.
- **DB-test strategy** (no DB in this env): the macro's *expansion* and the crate *compile* are the gates here; pure-logic DTO (de)serialization tests run; live CRUD behavior is verified on a real DB run. State this in the report.
- **Plain resources only** in P2a. geofences/routes (geometry-bearing) are P2b.

---

## Task 1: Standardize pagination on page/per_page

**Files:** `crates/koji-service/src/utils/pagination.rs`, `utils/api_response.rs` (`Meta::build`), `public/v2/jobs.rs` (`list_jobs`), `crates/koji-jobs/src/queue.rs` (`JobQueue::list`).

- [ ] **Step 1: Rework `Pagination` + `Meta::build` (TDD)**

Change `Pagination` to carry `page` (1-based, default 1, min 1) + `per_page` (default 50, clamp [1,500]); expose `page()` / `per_page()` / `offset()` (= `(page-1)*per_page`). Change `Meta::build` to `Meta::build(total: i64, page: i64, per_page: i64) -> Meta` (sets `page`, `per_page`, `total_pages = ceil(total/per_page)`, `has_next = page*per_page < total`, `has_prev = page > 1`). Update the existing pagination tests to the new shape (page 1 = first page).

Run: `cargo test -p koji-service --lib utils::pagination::tests 2>&1 | tail -8` → pass.

- [ ] **Step 2: Update `JobQueue::list` + `/jobs` list to page/per_page**

`JobQueue::list(page, per_page, status)` computes `offset = (page-1)*per_page` internally (keep the `(rows, total)` return). `list_jobs` reads `Pagination::{page,per_page}` and builds `Meta::build(total, page, per_page)`.

Run: `cargo test -p koji-jobs --no-run 2>&1 | tail -4` and `cargo test -p koji-service --lib 2>&1 | tail -4` → compile + pass.

- [ ] **Step 3: Commit**

```bash
git add crates/koji-service/src/utils/pagination.rs crates/koji-service/src/utils/api_response.rs crates/koji-service/src/public/v2/jobs.rs crates/koji-jobs/src/queue.rs
git commit -m "refactor(koji-service): standardize v2 pagination on page/per_page"
```

---

## Task 2: The `koji_resource!` proc-macro

**Files:** Create `crates/macros/src/` proc-macro `koji_resource` (new function-like proc-macro, exported from `macros`). Add a trybuild/expansion or apply-and-compile test.

### Macro contract (implement exactly)

**Invocation** (in koji-service):

```rust
koji_resource! {
    module: project,          // koji-db `db::project::Query`
    seg: "projects",          // URL segment
    create: {                 // Create DTO fields (required unless `?`)
        name: String,
        api_endpoint: Option<String>,
        api_key: Option<String>,
        scanner: bool,
        description: Option<String>,
    }
}
```

**Generates** a `pub(crate) mod project { … }` containing:

1. **`CreateProject`** — `#[derive(Debug, Deserialize, Serialize)] #[serde(rename_all = "camelCase")]` struct with the listed fields verbatim.
2. **`PatchProject`** — same fields, each wrapped in `Option<…>` (an already-`Option<T>` field stays `Option<T>`), `#[serde(default, skip_serializing_if = "Option::is_none")]` on each.
3. **Five handlers** (all `async fn(...) -> Result<HttpResponse, crate::utils::error::ServiceError>`):
   - `list(db, query: web::Query<crate::utils::pagination::Pagination>)` →
     `Query::paginate(&db.koji, /* AdminReqParsed from page/per_page */).await?` → `ApiResponse::success_paginated(results, Meta::build(total, page, per_page))`.
     *(Use the existing `Query::paginate`; construct its `AdminReqParsed` arg from `page`/`per_page` — see Task 2 step 2 note. If building `AdminReqParsed` is awkward, fall back to `Query::get_json_cache` + manual slice + `Meta::build`, and flag it.)*
   - `create(db, body: web::Json<CreateProject>)` → serialize the typed DTO to `serde_json::Value`, `Query::upsert_json_return(&db.koji, 0, value).await?` → `201` with `Location: /api/v2/{seg}/{id}` (read the new id from the returned record's `id`) + the record in the envelope.
   - `get_one(db, path: web::Path<String>)` → `Query::get_one_json(&db.koji, id).await` → `Ok` ⇒ envelope; map the koji-db "does not exist" `ModelError` to `ServiceError::NotFound { field: <seg-singular>, message }` (⇒ 404). *(koji-db's `crud_query`/hand `get_one` returns `ModelError::Geofence("Does not exist")` for misses — detect the not-found case; see step 3.)*
   - `update(db, path: web::Path<u32>, body: web::Json<PatchProject>)` → serialize DTO (omitting `None`), `Query::upsert_json_return(&db.koji, id, value).await?` → `200` envelope. 404 if the id doesn't exist (same not-found mapping).
   - `remove(db, path: web::Path<u32>)` → `Query::delete(&db.koji, id).await?`; if `rows_affected == 0` ⇒ `ServiceError::NotFound` (404); else `204 No Content` (empty body, NOT `{rows_affected}`).
4. **`pub(crate) fn scope() -> actix_web::Scope`** — `web::scope("/{seg}")` with `web::resource("")` (GET list, POST create) + `web::resource("/{id}")` (GET get_one, PATCH update, DELETE remove).

**Generated code resolves these at the call site** (emit fully-qualified): `crate::utils::api_response::ApiResponse`, `crate::utils::api_response::Meta`, `crate::utils::error::ServiceError`, `crate::utils::pagination::Pagination`, `koji_db::KojiDb`, `koji_db::db::{module}::Query`, `actix_web::{web, HttpResponse, http::StatusCode}`, `serde_json`. Follow the `crud_query`/`fort_query` precedent (unqualified only for call-site-prelude items; everything else fully qualified).

### Steps

- [ ] **Step 1: Add the proc-macro skeleton + a parse test**

In `crates/macros/src/lib.rs` add a `#[proc_macro] pub fn koji_resource(input: TokenStream) -> TokenStream`. Parse the invocation with `syn` (a custom `Parse` impl for the `module:`/`seg:`/`create:{…}` grammar). Start with a test (via `trybuild` pass-case, or—simpler—an in-crate `macro_rules!`-free expansion smoke by applying it in koji-service Task 3 and compiling). Recommended: drive correctness by **applying the macro to the first resource (Task 3) and compiling/​testing**, iterating the proc-macro until that resource's handlers compile + its DTO round-trip test passes. (Proc-macros are most reliably TDD'd against a real call site.)

- [ ] **Step 2: Build the macro to satisfy the contract**

Emit the `CreateX`/`PatchX` structs, the five handlers, and `scope()` per the contract. For the not-found mapping, add a small helper the generated `get_one`/`update` use: treat a `ModelError` whose `to_string()` contains `"Does not exist"` (koji-db's not-found sentinel) as `NotFound`; everything else flows through `ServiceError`'s `#[from] ModelError` (⇒ 500). Flag this string-sniff as a known shortcut — a cleaner `ModelError::NotFound` variant is a candidate follow-up (P2b or P4).

- [ ] **Step 3: Commit (after Task 3 proves it compiles)**

```bash
git add crates/macros/src/lib.rs
git commit -m "feat(macros): koji_resource! — typed CRUD DTOs + handlers + scope for plain v2 resources"
```

---

## Task 3: Apply to projects / properties / tile-servers

**Files:** Rewrite `crates/koji-service/src/public/v2/resources.rs` to use `koji_resource!` instead of `crud_resource!`; remove the old `crud_resource!` macro.

- [ ] **Step 1: Replace the three invocations**

```rust
koji_resource! {
    module: project, seg: "projects",
    create: { name: String, api_endpoint: Option<String>, api_key: Option<String>, scanner: bool, description: Option<String> }
}
koji_resource! {
    module: property, seg: "properties",
    create: { name: String, category: koji_db::Category, default_value: Option<String> }
}
koji_resource! {
    module: tile_server, seg: "tile-servers",
    create: { name: String, url: String }
}
```

*(Confirm the import path for `Category` — it lives in koji-db; if the property create should take a string and let koji-db parse it, use `String` and note it.)*

- [ ] **Step 2: DTO round-trip tests**

Add `#[cfg(test)]` tests in `resources.rs` asserting `CreateProject` / `PatchProject` (and the other two) deserialize from camelCase JSON and that `Patch*` accepts a partial body (omitted fields ⇒ `None`).

- [ ] **Step 3: Compile + lint + test**

Run: `cargo test -p koji-service --no-run 2>&1 | tail -8`, `cargo clippy -p koji-service --all-targets 2>&1 | tail -8`, `cargo test -p koji-service --lib 2>&1 | tail -6`
Expected: clean; DTO tests pass. The `scope()` names are unchanged (`resources::project::scope()` etc.), so `lib.rs` needs no change — confirm with `cargo build -p koji`.

- [ ] **Step 4: Commit**

```bash
git add crates/koji-service/src/public/v2/resources.rs
git commit -m "feat(koji-service): plain v2 CRUD via koji_resource! (typed DTOs, 201/Location, 204, 404)"
```

---

## Task 4: Verification gate

- [ ] **Step 1: Full build/lint/test**

```bash
cargo clippy -p macros -p koji-service -p koji-jobs --all-targets 2>&1 | tail -10
cargo test -p koji-service --lib 2>&1 | tail -6
cargo test -p koji-service --no-run 2>&1 | tail -4
cargo build -p koji 2>&1 | tail -3
```
Expected: clippy clean; lib tests pass; builds.

- [ ] **Step 2: Confirm the macro replaced the old one**

```bash
rg -n "crud_resource!" crates/koji-service/src || echo "CLEAN: crud_resource! gone"
rg -n "koji_resource!" crates/koji-service/src/public/v2/resources.rs
```
Expected: `CLEAN`; three `koji_resource!` invocations.

- [ ] **Step 3: DB-run note**

Live smoke on a real DB: `POST /api/v2/projects {"name":"x","scanner":false}` → `201`+`Location`; `GET /api/v2/projects?page=1&per_page=20` → enveloped list + `meta`; `GET /api/v2/projects/{id}` (404 for missing); `DELETE` → `204` (404 for missing). Recorded in the spec's P2 testing notes.

---

## Self-Review

**Spec coverage:** typed Create/Patch DTOs (§6.1/A5) ✓ · 201+Location/204/404 (§A9) ✓ · envelope-uniform (§2) ✓ · pagination (§6.3, page/per_page per the recorded deviation) ✓ · `#[koji_resource]` macro (§6.1) ✓ for plain resources (geometry resources = P2b). `ServiceError` adoption (§6.2) ✓.

**Placeholder scan:** the macro body is intentionally implementer-written from an exact contract (TDD against the call site) — not a placeholder; every generated item, status code, and koji-db call is specified. Fallbacks (paginate-arg construction, Category type, not-found sniff) name the exact alternative.

**Type consistency:** `Pagination::{page(),per_page(),offset()}`, `Meta::build(total,page,per_page)`, `ServiceError::NotFound{field,message}`, `Query::{paginate,get_json_cache,get_one_json,upsert_json_return,delete}`, `ApiResponse::{success,success_paginated,success_with_status}` — consistent with P0/P1 and the koji-db surface mapped by the scout.
