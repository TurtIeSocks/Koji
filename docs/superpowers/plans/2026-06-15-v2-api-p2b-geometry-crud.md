# v2 API — Phase 2b: Geometry CRUD (geofences / routes) — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:subagent-driven-development / executing-plans + test-driven-development. Steps use `- [ ]`.

**Goal:** Bring the hand-written geometry-bearing resources (geofences, routes) up to the v2 conventions: `ServiceError` (404 on missing), `201`+`Location`, `204` on delete, lightly-typed Create/Patch DTOs, and geometry reads through P0's `respond_geo` (`?format=`, envelope-uniform) instead of the legacy `utils::response::send`. Keep their geometry handling, the geofence hierarchy (`?depth`/`?level`), and the `/publish` actions.

**Architecture:** These handlers stay hand-written (they carry geometry, `?format`, hierarchy, and publish — beyond the `koji_resource!` macro's plain shape). The macro is NOT applied here; instead the same conventions are applied by hand. The big visible change: reads now return the v2 envelope (`{status:"ok",data:<GeoJSON>}`) by default, or a raw export with `?format=` — replacing the legacy `{message,status,status_code,data,stats}`. Safe: v2 has no consumers yet (the frontend rides v1).

**Tech Stack:** Rust, actix-web, sea-orm, serde, geojson.

## Delegate-mode decisions (recorded)

- **Lightly-typed DTOs.** `CreateGeofence`/`CreateRoute` type the scalar + link fields (`name`, `mode`, `parent`, `geofence_id`, `description`) and carry `geometry` as `serde_json::Value` (geojson) and geofence `projects`/`properties` as `Option<Vec<serde_json::Value>>`. Rationale: full geometry/join typing fights koji-db's upsert shape for marginal gain; scalar typing still gives boundary `400`s + documents the body. **If even the light DTO fights koji-db's expected shape, fall back to `web::Json<serde_json::Value>` for that body and still apply every other convention — flag it.** snake_case wire (per the P2a records-vs-config nuance).
- **`?rt=` → `?format=`; drop `?internal=`** (spec A8) from these public reads.
- **Hierarchy kept as-is.** `?depth=N` (cumulative subtree) and `?level=N` (exactly-N-below) stay distinct (NOT merged into `?descend` — they mean different things; merging loses information). `depth`+`level` together still → `400` (via `ServiceError::Invalid`).
- **Publish unchanged in behavior** — just routed through `ServiceError` for its `404`/`422`.
- **DB-test strategy:** no DB here → compile/clippy gates + pure DTO (de)serialization tests; live geometry CRUD on a real-DB run.

---

## Task 1: geofences.rs — conventions + DTOs

**Files:** Modify `crates/koji-service/src/public/v2/geofences.rs`. Read the current handlers first.

- [ ] **Step 1: Add the typed DTOs (TDD)**

```rust
#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct CreateGeofence {
    pub name: String,
    pub mode: Option<String>,
    pub geometry: serde_json::Value,
    pub parent: Option<u32>,
    #[serde(default)]
    pub projects: Vec<serde_json::Value>,
    #[serde(default)]
    pub properties: Vec<serde_json::Value>,
}

#[derive(Debug, Deserialize, Serialize, Default)]
#[serde(default)]
pub(crate) struct PatchGeofence {
    pub name: Option<String>,
    pub mode: Option<String>,
    pub geometry: Option<serde_json::Value>,
    pub parent: Option<u32>,
    pub projects: Option<Vec<serde_json::Value>>,
    pub properties: Option<Vec<serde_json::Value>>,
}
```

Add a test: `CreateGeofence` deserializes from a snake_case body with a geojson `geometry`; `PatchGeofence` accepts `{}` (all `None`). Confirm the field set matches what koji-db's geofence `upsert`/`to_geofence` reads (read it; adjust names to match, e.g. if it expects `geo_type` or a nested shape).

- [ ] **Step 2: Rewrite the read handlers (`list`, `get_one`) → `respond_geo` + `ServiceError`**

- Handlers return `Result<HttpResponse, ServiceError>`.
- Parse `?format=` (fall back to `rt` for one release if trivial; else just `format`) via the existing `get_return_type` (default `featurecollection` for list, `feature` for get_one).
- Keep the `?depth`/`?level` hierarchy branch (`Query::descendants` with the `Anchor`/`HierarchySpec`); `depth`+`level` both present → `Err(ServiceError::Invalid { field: Some("hierarchy".into()), message: "depth and level are mutually exclusive".into() })`.
- Replace the terminal `utils::response::send(coll, rt, None, false, area)` with `crate::utils::format::respond_geo(coll, rt)`.
- `get_one`: a missing geofence (the `Query::get_one_koji`/`descendants` not-found) → `Err(ServiceError::NotFound { field: "geofence", message: format!("no geofence {id}") })`.
- Drop the `?internal=` handling.

- [ ] **Step 3: Rewrite the write handlers (`create`, `update`, `remove`)**

- `create(db, body: web::Json<CreateGeofence>)`: serialize the DTO to `Value`, `Query::upsert_json_return(&db.koji, 0, value).await?`; `201` with `Location: /api/v2/geofences/{id}` (id from the returned record) + the record in the envelope.
- `update(db, path: web::Path<u32>, body: web::Json<PatchGeofence>)`: existence pre-check (`Query::get_one(id)` → `NotFound`⇒404), then upsert(id) → `200` envelope. (koji-db upsert is insert-on-missing, so the pre-check enforces the 404 contract — mirror P2a's `update`.)
- `remove(db, path)`: `Query::delete(id)`; `rows_affected == 0` ⇒ `ServiceError::NotFound`; else `204` no content.

- [ ] **Step 4: `publish` → `ServiceError`**

Keep the publish logic; convert its error paths: unknown geofence ⇒ `ServiceError::NotFound`; no `dragonite_area_id` ⇒ `ServiceError::Unprocessable { field: Some("dragonite_area_id".into()), message: "geofence is not linked to a Dragonite area".into() }`; success stays `202` with `{geofence, event_id, topic, dragonite_area_id}`.

- [ ] **Step 5: Compile, lint, test, commit**

Run: `cargo test -p koji-service --no-run 2>&1 | tail -8`, `cargo clippy -p koji-service --all-targets 2>&1 | tail -8`, `cargo test -p koji-service --lib geofences 2>&1 | tail -8`
Expected: clean; DTO tests pass.

```bash
git add crates/koji-service/src/public/v2/geofences.rs
git commit -m "feat(koji-service): geofences v2 conventions — typed DTOs, ServiceError 404/204/Location, respond_geo ?format"
```

---

## Task 2: routes.rs — conventions + DTOs

**Files:** Modify `crates/koji-service/src/public/v2/routes.rs` (no hierarchy; otherwise mirrors Task 1).

- [ ] **Step 1: DTOs (TDD)**

```rust
#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct CreateRoute {
    pub geofence_id: u32,
    pub name: String,
    pub mode: Option<String>,
    pub description: Option<String>,
    pub geometry: serde_json::Value,
}

#[derive(Debug, Deserialize, Serialize, Default)]
#[serde(default)]
pub(crate) struct PatchRoute {
    pub geofence_id: Option<u32>,
    pub name: Option<String>,
    pub mode: Option<String>,
    pub description: Option<String>,
    pub geometry: Option<serde_json::Value>,
}
```

Test deserialization (snake_case); adjust to koji-db route `upsert`/`to_route`'s expected keys.

- [ ] **Step 2: Reads (`list`, `get_one`) → `respond_geo` + `ServiceError`**

`?format=` (default `featurecollection`/`feature`), `Query::as_koji_collection` / `get_one_koji` → `respond_geo(coll, rt)`. Missing route in `get_one` ⇒ `ServiceError::NotFound { field: "route", … }`. Drop `?internal=`.

- [ ] **Step 3: Writes (`create`, `update`, `remove`)**

Same pattern as geofences: typed body → upsert; `201`+`Location: /api/v2/routes/{id}`; `update` existence-pre-check ⇒ 404; `remove` ⇒ `204` / 404.

- [ ] **Step 4: `publish` → `ServiceError`**

Unknown route ⇒ `NotFound`; route's geofence has no `dragonite_area_id` ⇒ `Unprocessable`; success `202` `{route, event_id, topic, dragonite_area_id, mode}`.

- [ ] **Step 5: Compile, lint, test, commit**

Run the same gate as Task 1 Step 5 (with `routes` filter).

```bash
git add crates/koji-service/src/public/v2/routes.rs
git commit -m "feat(koji-service): routes v2 conventions — typed DTOs, ServiceError 404/204/Location, respond_geo ?format"
```

---

## Task 3: Verification gate

- [ ] **Step 1: Build / lint / test**

```bash
cargo clippy -p koji-service --all-targets 2>&1 | tail -10
cargo test -p koji-service --lib 2>&1 | tail -6
cargo test -p koji-service --no-run 2>&1 | tail -4
cargo build -p koji 2>&1 | tail -3
```
Expected: clean; lib tests pass (P2a's 53 + the new DTO tests).

- [ ] **Step 2: Confirm legacy `send` is gone from these reads**

```bash
rg -n "utils::response::send|\\?internal|ApiResponse::fail|ErrorInternalServerError" crates/koji-service/src/public/v2/geofences.rs crates/koji-service/src/public/v2/routes.rs || echo "CLEAN: legacy send/internal/fail/map_err gone from geo CRUD"
rg -n "respond_geo|ServiceError" crates/koji-service/src/public/v2/geofences.rs crates/koji-service/src/public/v2/routes.rs | head
```
Expected: first prints `CLEAN`; second shows `respond_geo` + `ServiceError` usage.

- [ ] **Step 3: DB-run note**

Live smoke (real DB): `GET /api/v2/geofences` (FeatureCollection in envelope), `?format=sql` (raw text/plain), `?depth=1`/`?level=1` hierarchy, `GET /api/v2/geofences/{id}` 404 for missing, `POST` → 201+Location, `DELETE` → 204, `POST /{id}/publish` (202 / 422 unlinked / 404). Same for routes. Recorded in the spec's P2 testing notes.

---

## Self-Review

**Spec coverage:** geometry reads envelope-uniform via `respond_geo`/`?format=` (§2, §6.4) ✓ · `ServiceError` 404/204/201+Location (§A9, §6.2) ✓ · lightly-typed DTOs (§A5, pragmatic for geometry) ✓ · `?internal` dropped (§A8) ✓ · publish preserved (§4.2) ✓ · hierarchy kept (deviation from §4.2's `?descend` merge — documented: distinct semantics). 

**Placeholder scan:** none — per-handler target behavior + DTO shapes + status codes specified; the Value-fallback names the exact alternative; koji-db key-matching is an explicit read-and-match step, not a TODO.

**Type consistency:** `respond_geo(KojiGeometryCollection, ReturnTypeArg)`, `ServiceError::{NotFound{field,message}, Invalid{field,message}, Unprocessable{field,message}}`, `Query::{get_one, get_one_koji, as_koji_collection, descendants, upsert_json_return, delete}`, `get_return_type(String,&ReturnTypeArg)` — consistent with P0/P1/P2a + the scout's koji-db map.
