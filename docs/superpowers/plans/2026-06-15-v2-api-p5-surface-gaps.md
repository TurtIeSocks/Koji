# v2 API — Phase 5: Surface completion (gap endpoints) — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:subagent-driven-development / executing-plans + test-driven-development. Steps use `- [ ]`.

**Goal:** Fill the v2 surface gaps the frontend migration revealed, so the frontend can move off v1/`/internal` entirely (P6). Three backend additions: polygon area calc, arbitrary-area scanner-data (+ stats), and the app-config blob. All ported from existing v1/`/internal` handlers.

**Architecture:** Each new endpoint ports its v1/`/internal` sibling's logic onto the v2 conventions (`ServiceError`, envelope, typed body). No new domain logic — these are surface-shape ports of working handlers.

**Tech Stack:** Rust, actix-web, sea-orm, geojson, koji-scanner.

## Delegate-mode decisions (recorded)

- **Arbitrary-area scanner-data is a POST** (`/api/v2/scanner-data/{category}`, body carries the area/bbox) — a GET can't carry a drawn polygon. This coexists with P3's `GET /api/v2/geofences/{id}/scanner-data` (saved-fence convenience). Category stays in the path (matches the old `/internal/data/area/{category}`).
- **`GET /api/v2/config`** ports the existing `/config` (`private/misc.rs::config`) `ConfigResponse` into the v2 envelope — it's the app bootstrap blob (map center, tile server, plugin lists, login state), distinct from `/auth/me` (auth state only).
- **Area unit** matches v1 `calculate_area` exactly (don't reinterpret the unit — port the number the frontend already expects).
- **NOT in P5 (flagged for the user, domain-ambiguous):** `save-scanner`/`push`→`publish` equivalence, and `/internal/routes/from_scanner` (external scanner-sourced routes). The P6 frontend migration maps these best-guess and flags them; they are NOT resolved blind here.
- **DB-test strategy:** no DB here → compile/clippy + pure-logic tests; live behavior on a real-DB run.

---

## Task 1: `POST /api/v2/geometry/area`

**Files:** Modify `crates/koji-service/src/public/v2/geometry.rs`. Read `crates/koji-service/src/public/v1/calculate.rs::calculate_area` (the v1 handler to port).

- [ ] **Step 1: Port the handler**

Add an `area` handler to `geometry.rs`: `POST /geometry/area`, body = the same area input the other `/geometry/*` transforms take (a typed request carrying `area`, e.g. reuse `SimplifyReq` or a small `AreaReq { area: GeoInput }`), compute the area via the same geometry routine v1 `calculate_area` uses (Chamberlain-Duquette unsigned — find the exact fn it calls), return `ApiResponse::success(json!({ "area": <value> }))`. Signature `-> Result<HttpResponse, ServiceError>`; bad geometry ⇒ `ServiceError::Invalid`. Add it to the `/geometry` `scope()`.

- [ ] **Step 2: Test + commit**

Pure-logic test: feed a known polygon, assert the area value matches v1's output (compute the expected from the same routine). Compile-gate.

```bash
git add crates/koji-service/src/public/v2/geometry.rs
git commit -m "feat(koji-service): POST /api/v2/geometry/area (port v1 calc/area)"
```

---

## Task 2: `POST /api/v2/scanner-data/{category}` + `/stats`

**Files:** Modify `crates/koji-service/src/public/v2/scanner_data.rs`. Read `crates/koji-service/src/private/points.rs` (`by_area`, `area_stats`, and the `bound` variant) — the `/internal/data/*` handlers to port.

- [ ] **Step 1: Add the arbitrary-area handlers**

- `POST /api/v2/scanner-data/{category}` — body `{ area?: <GeoInput/FeatureCollection>, bbox?: {minLat,minLon,maxLat,maxLon}, lastSeen?: u32, tth?: <SpawnpointTth> }`. Resolve points within the supplied area (or bbox) for `{category}` via the same koji-scanner query the `/internal/data/area`(+`bound`) handler uses. Return `ApiResponse::success(json!({ "points": <SingleVec> }))`. Unknown category ⇒ `ServiceError::Invalid { field: Some("category".into()), … }`.
- `POST /api/v2/scanner-data/{category}/stats` — same body → `ApiResponse::success(json!({ "total": <usize> }))` (port `area_stats`).
- Add a `pub(crate) fn scope() -> actix_web::Scope` = `web::scope("/scanner-data")` with `web::resource("/{category}")` (POST) + `web::resource("/{category}/stats")` (POST). (The geofence-nested GET from P3 stays where it is, in `geofences::scope()`.)
- All handlers `-> Result<HttpResponse, ServiceError>`.

- [ ] **Step 2: Compile + commit**

```bash
git add crates/koji-service/src/public/v2/scanner_data.rs
git commit -m "feat(koji-service): POST /api/v2/scanner-data/{category}(+/stats) — arbitrary-area markers (port /internal/data)"
```

---

## Task 3: `GET /api/v2/config`

**Files:** Create `crates/koji-service/src/public/v2/config.rs` (or add to an existing v2 module); modify `mod.rs`. Read `crates/koji-service/src/private/misc.rs::config` (builds `ConfigResponse`) + `utils/response.rs::ConfigResponse`.

- [ ] **Step 1: Port the config blob**

`GET /api/v2/config` builds the same `ConfigResponse` (`start_lat`, `start_lon`, `tile_server`, `logged_in`, `dangerous`, `route_plugins`, `clustering_plugins`, `bootstrap_plugins`) the `/config` handler does, returned via `ApiResponse::success(...)`. Reuse the existing `ConfigResponse` struct + the same data sources (env / session / plugin registry). Provide a `scope()` or a single registered handler. Signature `-> Result<HttpResponse, ServiceError>`.

- [ ] **Step 2: Compile + commit**

```bash
git add crates/koji-service/src/public/v2/config.rs crates/koji-service/src/public/v2/mod.rs
git commit -m "feat(koji-service): GET /api/v2/config (app bootstrap blob)"
```

---

## Task 4: Wire + verify

**Files:** Modify `crates/koji-service/src/lib.rs`.

- [ ] **Step 1: Mount the new scopes**

In the `/v2` scope add `.service(public::v2::scanner_data::scope())` and the `/config` registration. (`/geometry/area` rides the existing `geometry::scope()`.)

- [ ] **Step 2: Build / lint / test**

```bash
cargo clippy -p koji-service --all-targets 2>&1 | tail -10
cargo test -p koji-service --lib 2>&1 | tail -6
cargo build -p koji 2>&1 | rg "Finished|error" | head
```
Expected: clean; lib tests pass (P4 = 79 + new); builds.

- [ ] **Step 3: Confirm the surface**

```bash
rg -n "geometry/area|scanner_data::scope|/api/v2/config|\"/config\"" crates/koji-service/src/lib.rs crates/koji-service/src/public/v2/geometry.rs crates/koji-service/src/public/v2/scanner_data.rs
```
Expected: the three new mounts present.

- [ ] **Step 3 commit**

```bash
git add crates/koji-service/src/lib.rs
git commit -m "feat(koji-service): mount /scanner-data + /config v2 scopes"
```

---

## Self-Review

**Spec coverage:** closes the frontend-revealed gaps — area calc, arbitrary-area scanner-data (+stats), app-config — all ported onto v2 conventions. (`save-scanner`/`push`/`from_scanner` deliberately deferred + flagged, not guessed.)

**Placeholder scan:** none — each task ports a named v1/`/internal` handler; "read the v1 handler" is the grounding step; DB-only verification flagged.

**Type consistency:** `ServiceError::{Invalid,…}`, `ApiResponse::success`, `ConfigResponse`, `GeoInput`/`SingleVec`/`SpawnpointTth`, the koji-scanner area query — consistent with P0–P4 + the v1 handlers being ported.
