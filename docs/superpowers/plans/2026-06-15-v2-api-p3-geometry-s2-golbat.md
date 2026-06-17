# v2 API — Phase 3: Geometry transforms / S2 / golbat-data — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:subagent-driven-development / executing-plans + test-driven-development. Steps use `- [ ]`.

**Goal:** Regroup the `/api/v2/geo/*` grab-bag into honest namespaces and re-nest golbat-data under its geofence:
- `/api/v2/geometry/{convert,simplify,merge-points}` (was `/geo/convert|simplify|merge-points`) — typed bodies, `respond_geo` (`?format=`), `ServiceError`.
- `/api/v2/s2/{circle-coverage,cell-coverage,polygons,{level}}` (lifted out of `/geo/s2/*`) — `ServiceError`, envelope kept.
- `/api/v2/geofences/{id}/golbat-data?category=&lastSeen=` (was `/golbat-data/{category}?instance=`) — id-consistent sub-resource.

**Architecture:** `geo.rs` splits into `geometry.rs` (the three transforms) + `s2.rs` (the four S2 ops), each with a `scope()`; `geo.rs` is deleted. The transforms already deserialize typed requests (`ConvertReq`/`SimplifyReq`/`MergePointsReq` in `requests/ops.rs`) — they just need `respond_geo` + `ServiceError`. golbat-data moves into the geofences scope, keyed by the geofence path id (id/name) instead of an `?instance=` query.

**Tech Stack:** Rust, actix-web, sea-orm, serde, geojson, S2.

## Delegate-mode decisions (recorded)

- **Split `geo.rs` → `geometry.rs` + `s2.rs`.** The two are unrelated (geometry transforms vs S2 cells); separate modules + namespaces are clearer than one `/geo` bag.
- **Transforms honor `?format=`** (default from the body's `output.return_type`, else `featurecollection`) and go through `respond_geo` (envelope GeoJSON / raw export) — same negotiation as the geometry reads in P2b.
- **S2 endpoints keep the enveloped `ApiResponse`** shape (their output is cell-id arrays / coverage objects / polygons, not negotiable GeoJSON) but adopt `ServiceError`.
- **golbat-data `lastSeen` query is camelCase** (a config-style query param, like calc args), defaulting to 0; `category` is a query param. The geofence `{id}` (id or name) supplies the area.
- **DB-test strategy:** no DB here → compile/clippy + pure-logic tests; live behavior on a real-DB run.

---

## Task 1: `geometry.rs` — the three transforms

**Files:** Create `crates/koji-service/src/public/v2/geometry.rs`; modify `public/v2/mod.rs`. Read the current `geo.rs` transform handlers first.

- [ ] **Step 1: Move + convert the three handlers**

For each of `convert` / `simplify` / `merge-points`: keep the existing typed body (`ConvertReq`/`SimplifyReq`/`MergePointsReq`) and the existing geometry logic, but:
- signature → `async fn(...) -> Result<HttpResponse, ServiceError>`;
- resolve the return type: `?format=` query if present, else the body's `output.return_type` (the existing default — `default_return_type()` from the area container), then terminal `crate::utils::format::respond_geo(collection, rt)` instead of `utils::response::send`;
- map any error (geometry parse, area resolution) through `ServiceError` (`Invalid` for bad input, `internal` for plumbing).

Provide a `pub(crate) fn scope() -> actix_web::Scope` = `web::scope("/geometry")` with `convert` / `simplify` / `merge-points` POST resources.

- [ ] **Step 2: Test + commit**

Pure-logic tests where possible (the transforms' geometry helpers, or a request-deser test). Compile-gate the rest.

Run: `cargo test -p koji-service --no-run 2>&1 | tail -6`, `cargo clippy -p koji-service --all-targets 2>&1 | tail -6`

```bash
git add crates/koji-service/src/public/v2/geometry.rs crates/koji-service/src/public/v2/mod.rs
git commit -m "feat(koji-service): /geometry/* transforms — respond_geo ?format, ServiceError (from /geo)"
```

---

## Task 2: `s2.rs` — the four S2 ops

**Files:** Create `crates/koji-service/src/public/v2/s2.rs`; modify `mod.rs`. Read the current `geo.rs` S2 handlers.

- [ ] **Step 1: Move + convert**

Move `circle_coverage` / `cell_coverage` / `cell_polygons` / `s2_cells` ({level} path) verbatim except: signature → `Result<HttpResponse, ServiceError>`; keep the `ApiResponse::success(...)` payloads; map errors through `ServiceError`. `pub(crate) fn scope() -> actix_web::Scope` = `web::scope("/s2")` with the four POST resources (`/circle-coverage`, `/cell-coverage`, `/polygons`, `/{level}`).

- [ ] **Step 2: Delete `geo.rs`**

Remove `crates/koji-service/src/public/v2/geo.rs` and its `mod geo;` line (the transforms + S2 now live in `geometry`/`s2`).

- [ ] **Step 3: Compile + commit**

Run: `cargo test -p koji-service --no-run 2>&1 | tail -6`, `cargo clippy -p koji-service --all-targets 2>&1 | tail -6`

```bash
git add crates/koji-service/src/public/v2/s2.rs crates/koji-service/src/public/v2/mod.rs crates/koji-service/src/public/v2/geo.rs
git commit -m "feat(koji-service): /s2/* ops lifted out of /geo; delete geo.rs"
```

---

## Task 3: golbat-data → geofence sub-resource

**Files:** Modify `crates/koji-service/src/public/v2/golbat_data.rs`. Read it first.

- [ ] **Step 1: Re-key the handler on the geofence path id**

New shape: `GET /api/v2/geofences/{id}/golbat-data?category=&lastSeen=`.
- `path: web::Path<String>` = the geofence id-or-name (was the `?instance=` query).
- `query`: `{ category: String, #[serde(rename = "lastSeen", default)] last_seen: u32 }` (camelCase wire).
- Resolve the geofence's area by the path id (reuse the same area-resolution the handler used for `?instance=`, keyed on id/name); a missing geofence ⇒ `ServiceError::NotFound { field: "geofence", … }`. An unknown `category` ⇒ `ServiceError::Invalid { field: Some("category".into()), … }` (preserve the current category validation).
- Keep the `ApiResponse::success({ points })` payload. Signature → `Result<HttpResponse, ServiceError>`.

- [ ] **Step 2: Compile + commit**

Run: `cargo test -p koji-service --no-run 2>&1 | tail -6`

```bash
git add crates/koji-service/src/public/v2/golbat_data.rs
git commit -m "feat(koji-service): golbat-data → /geofences/{id}/golbat-data (id-keyed), ServiceError"
```

---

## Task 4: Rewire `/v2` scope in `lib.rs`

**Files:** Modify `crates/koji-service/src/lib.rs`; possibly `public/v2/geofences.rs` (to host the golbat-data sub-route).

- [ ] **Step 1: Swap the registrations**

In the `web::scope("/v2")` block:
- remove `.service(public::v2::geo::scope())` → add `.service(public::v2::geometry::scope())` and `.service(public::v2::s2::scope())`.
- remove `.service(public::v2::golbat_data::golbat_data)` (the old flat mount).
- mount golbat-data under geofences: in `geofences::scope()` add `.service(web::resource("/{id}/golbat-data").route(web::get().to(crate::public::v2::golbat_data::golbat_data)))` (alongside `/{id}` and `/{id}/publish`). Confirm the `golbat_data` handler is `pub(crate)` and reachable.

- [ ] **Step 2: Build the whole bin + commit**

Run: `cargo build -p koji 2>&1 | tail -6`, `cargo clippy -p koji-service --all-targets 2>&1 | tail -6`

```bash
git add crates/koji-service/src/lib.rs crates/koji-service/src/public/v2/geofences.rs
git commit -m "feat(koji-service): rewire /v2 — /geometry + /s2 scopes, golbat-data under geofences"
```

---

## Task 5: Verification gate

- [ ] **Step 1: Build / lint / test**

```bash
cargo clippy -p koji-service --all-targets 2>&1 | tail -10
cargo test -p koji-service --lib 2>&1 | tail -6
cargo test -p koji-service --no-run 2>&1 | tail -4
cargo build -p koji 2>&1 | tail -3
```
Expected: clean; lib tests pass (P2 = 68 + any new); builds.

- [ ] **Step 2: Confirm the new surface**

```bash
rg -n "mod geo\b|public::v2::geo::|/geo/" crates/koji-service/src || echo "CLEAN: /geo gone"
rg -n "geometry::scope|s2::scope|golbat-data" crates/koji-service/src/lib.rs crates/koji-service/src/public/v2/geofences.rs
```
Expected: `CLEAN`; the new `/geometry`, `/s2`, and `/{id}/golbat-data` registrations present.

- [ ] **Step 3: DB-run note**

Live smoke (real DB): `POST /api/v2/geometry/convert` (GeoJSON in envelope; `?format=sql` raw); `POST /api/v2/s2/15` (cells); `GET /api/v2/geofences/{id}/golbat-data?category=pokestop&lastSeen=0` (points; 404 unknown fence; 400 bad category). Recorded in the spec's P3 testing notes.

---

## Self-Review

**Spec coverage:** `/geometry/*` regroup (§4.3, §A12) ✓ · `/s2/*` lifted (§4.4, §A12) ✓ · golbat-data sub-resource (§4.5, §A11) ✓ · transforms envelope/`?format` (§6.4) ✓ · `ServiceError` (§6.2) ✓. `/geo/*` deleted.

**Placeholder scan:** none — per-handler moves + signature changes + the exact new routes are specified; "read the current handler" is a grounding step, not a TODO; DB-only verification flagged.

**Type consistency:** `respond_geo(KojiGeometryCollection, ReturnTypeArg)`, `ServiceError::{NotFound,Invalid,internal}`, `ApiResponse::success`, `ConvertReq`/`SimplifyReq`/`MergePointsReq` + their `default_return_type()`, `CoverageArgs`/`BoundsArg` — all consistent with P0–P2 + the scout's map.
