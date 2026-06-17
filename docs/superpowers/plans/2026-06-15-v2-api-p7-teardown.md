# v2 API — Phase 7: v1 / internal / config teardown — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:executing-plans. Verified by the whole-workspace build (deleting v1 surfaces every v2→v1 dangling ref). Steps use `- [ ]`.

**Goal:** Full carnage — delete the legacy `/api/v1/*`, `/internal/*`, and `/config/*` surfaces now that the frontend rides v2 (P6). Move the one survivor (Nominatim) onto v2. Leave a single coherent `/api/v2` surface (+ health + static SPA).

**Architecture:** The frontend already calls v2 only, so v1/`/internal`/`/config` are dead-caller code. Deleting them surfaces every place v2 still reaches back into v1 (notably `LegacyArgs::into_calc_request` referenced from `calc.rs`); each is removed/inlined. Then prune the now-dead legacy response helpers.

**Tech Stack:** Rust, actix-web; one small TS edit (Nominatim).

## Delegate-mode decisions (recorded)

- **Move Nominatim → `/api/v2/nominatim`** (don't keep a lone `/config/nominatim` survivor) and update `client/.../Nominatim.tsx`, so `/config/*` deletes cleanly.
- **Keep workspace green at every step** — delete a unit, fix the dangling refs it surfaces, repeat. The build is the gate.
- **Prune dead legacy response code** only after confirming zero callers (`Response`/`send`/`ok_response` were v1's; `response_body` + `ConfigResponse` are still used by v2 — keep those).

---

## Task 1: Delete `/internal/*` (data + admin)

**Files:** delete `crates/koji-service/src/private/points.rs`, `private/admin.rs`, `private/geofence_project.rs`; modify `private/mod.rs`, `lib.rs`.

- [ ] **Step 1** — Remove the three modules + their `mod` decls + the `/internal` scope registration in `lib.rs` (the `web::scope("/internal")` block). Fix any dangling refs (these were self-contained admin handlers; unlikely to be referenced by v2).
- [ ] **Step 2** — `cargo build -p koji 2>&1 | rg "Finished|error"` → `Finished`. Commit.

```bash
git commit -am "refactor(koji-service): delete /internal/{data,admin} (subsumed by v2 CRUD + golbat-data)"
```

---

## Task 2: Delete `/api/v1/*` + `LegacyArgs`

**Files:** delete the whole `crates/koji-service/src/public/v1/` tree; modify `public/mod.rs`, `lib.rs`, and `public/v2/calc.rs` (remove the `LegacyArgs` mapping path).

- [ ] **Step 1** — Remove the `web::scope("/v1")` block in `lib.rs` and the `mod v1;` decl. Delete `public/v1/`.
- [ ] **Step 2** — Fix the v2→v1 dangling refs the deletion surfaces. Known: `public/v2/calc.rs` references `LegacyArgs::into_calc_request` (the v1 flat-wire bridge) — remove that path; v2 enqueues a `CalcRequest` directly, so the handler no longer needs the legacy mapping. Update the module doc comment that mentions it. Resolve any others the compiler flags.
- [ ] **Step 3** — `cargo build -p koji` + `cargo test -p koji-service --no-run` → green. Commit.

```bash
git commit -am "refactor(koji-service): delete /api/v1/* + LegacyArgs (v2 is the surface)"
```

---

## Task 3: Move Nominatim → v2; delete `/config/*`

**Files:** `crates/koji-service/src/public/v2/` (new `nominatim` handler or fold into `config.rs`/`misc`), `private/misc.rs` (delete login/logout/config/nominatim), `lib.rs`, `client/src/.../Nominatim.tsx`.

- [ ] **Step 1** — Add `GET /api/v2/nominatim?query=…` (port `private::misc::search_nominatim`) returning the envelope. Register in the `/v2` scope.
- [ ] **Step 2** — Delete the `/config` scope + `private/misc.rs`'s `login`/`logout`/`config`/`search_nominatim` (the frontend now uses `/api/v2/auth/*` + `/api/v2/config`). If `private/misc.rs` becomes empty, delete it + its `mod` decl. Keep `ConfigResponse` (moved/owned where `/api/v2/config` uses it).
- [ ] **Step 3** — Update `client/src/.../Nominatim.tsx`: `/config/nominatim?query=` → `/api/v2/nominatim?query=` (remove its `TODO(v2-verify)`).
- [ ] **Step 4** — `cargo build -p koji` green; `client/node_modules/.bin/tsc --noEmit -p client` → 0 errors. Commit.

```bash
git commit -am "refactor: Nominatim → /api/v2/nominatim; delete /config/* (v2 auth+config own it)"
```

---

## Task 4: Prune dead legacy response helpers

**Files:** `crates/koji-service/src/utils/response.rs` + anywhere flagged dead.

- [ ] **Step 1** — With v1 gone, check callers of `utils::response::{Response, send, ok_response}` (`rg`). Remove the ones with zero callers. **Keep** `response_body` (used by `respond_geo`) and `ConfigResponse` (used by `/api/v2/config`). Remove any other now-orphaned v1-era helpers the compiler/clippy flags as dead (`ApiResponse::fail`/`first_field_message` if now caller-less, etc.).
- [ ] **Step 2** — `cargo clippy -p koji-service --all-targets` → no dead-code warnings (or justified `#[allow]`). Commit.

```bash
git commit -am "refactor(koji-service): prune dead legacy response helpers post-v1"
```

---

## Task 5: Verification gate

- [ ] **Step 1** — Full workspace green:

```bash
cargo clippy --workspace --all-targets 2>&1 | rg "error|warning: unused|warning: dead" | rg -v "num-bigint-dig|proc-macro-error2" | head
cargo test -p koji-service --lib 2>&1 | rg "test result"
cargo build -p koji 2>&1 | rg "Finished|error"
cargo test -p koji-service -p koji-jobs --no-run 2>&1 | tail -3
```
Expected: clippy clean (bar the transitive note); lib tests pass (P5 = 85); builds.

- [ ] **Step 2** — Confirm the surface is single:

```bash
rg -n "scope\(\"/v1\"\)|scope\(\"/internal\"\)|scope\(\"/config\"\)|public::v1|private::points|private::admin" crates/koji-service/src || echo "CLEAN: v1//internal//config gone"
rg -n "/api/v1/|/internal/" client/src || echo "CLEAN: frontend has no v1//internal calls"
```
Expected: both `CLEAN`.

- [ ] **Step 3** — Commit any final tidy.

---

## Self-Review

**Spec coverage:** single-surface north star (§1, §A2) achieved — v1 + `/internal` + `/config` deleted, `LegacyArgs` gone, Nominatim on v2. Workspace compiles + the 85 backend lib tests still pass (v2 untouched by the deletion).

**Placeholder scan:** none — each deletion target is named; "fix the dangling refs the compiler surfaces" is the defined method (the build is the oracle).

**Risk note:** the only non-mechanical bit is the `calc.rs` `LegacyArgs` removal — confirm v2's calc path is self-contained (it enqueues `CalcRequest` directly, resolved in P1) before deleting the bridge.
