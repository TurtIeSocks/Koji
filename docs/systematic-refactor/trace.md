# Koji V2 — Phase 1 Trace

> Trace depth: **flow / module hybrid**. Repo ≈ 19k LOC Rust (10k–100k band). Per user
> direction, algorithm *math* is NOT traced; crate boundaries, the `Args` super-struct,
> the HTTP surface, golbat integration, plugin system, and concurrency model ARE.

## Signal tags

`[HOT]` high churn · `[COMPLEX]` large/tangled · `[DEPRECATED]` dead-ish · `[COUPLED]` separation-of-concerns violation · `[RDM]` removal target

---

## 1. Repo shape

```
Koji/
  server/            <-- Cargo workspace root (NOT repo root)  [proper-workspace target]
    Cargo.toml       package `koji` v1.5.4, edition 2024; members: . algorithms api macros migration model nominatim
    src/main.rs      bin `koji` — dotenv + env_logger + api::start()
    algorithms/      clustering, routing, bootstrap, s2, plugin, stats   [HOT]
    api/             actix-web server: public/v1 + private/internal      [HOT]
    macros/          proc-macros
    migration/       sea-orm migrations (+ its own bin main.rs)
    model/           db entities + api DTOs + utils ALL MIXED            [COMPLEX][COUPLED]
    nominatim/       standalone Nominatim geocoding client
  client/            frontend (React) — DO NOT TOUCH phase 1; note touch-points
  or-tools/          Google OR-Tools (C++ VRP solver) — vendored
  docs/
```

**Crate dependency graph**
```
koji(bin) → api
api        → algorithms, migration, model, nominatim
algorithms → model, macros
model      → (base)        ← everything depends on this; it is the dumping ground
nominatim  → (standalone)
macros     → (standalone)
```

**Binaries today:** `koji` (web server), `migration` (migration runner). User wants a 3rd: an **algorithms CLI**.

---

## 2. `model` crate — the nightmare  [COMPLEX][COUPLED]

Three concerns fused in one crate. `model/src/`:

- `db/` — 13 **sea-orm-codegen** entities (area, geofence, geofence_project, geofence_property, gym, instance, pokestop, project, property, route, spawnpoint, station, tile_server) + `sea_orm_active_enums.rs` (`Type`, `Category`, `FenceMode`) + hand-written query structs in `db/mod.rs` (`NameId`, `AreaRef`, `PaginateResults<T>`, `GenericData`, `InstanceParsing`, `RdmInstance*` …).
- `api/` — request/response DTOs + ~18 conversion traits (`ToCollection`, `ToFeature`, `ToSingleVec`, `ToPoracle`, `ToSql` …), `GeoFormats` enum, `BBox`, and the `Args` super-struct.
- `utils/` — `get_database_struct()` (DB bootstrap + golbat-type detection), `name_modifier()`, `sql_raw()`, enum lookups.

**Separation-of-concerns violations (evidence):**
| Site | Problem |
|---|---|
| `db/geofence.rs:30-43` | sea-orm `Model` also derives `Serialize/Deserialize` → DB entity doubles as API DTO |
| `db/geofence.rs:5-14`, `db/area.rs:6-9` | DB entity files import `api::{GeoFormats, ToCollection, ToText, args::*}` → DB layer depends on API layer |
| `db/area.rs:41-73` | `Model::to_feature()` — DB entity *produces* a GeoJSON API Feature |
| `db/mod.rs:94-99` | `RdmInstanceArea` variants reference `api::{point_struct,single_struct,multi_struct}` → domain couples db→api geometry |
| `api/mod.rs:1-4`, `api/text.rs:16-46` | API layer imports db `Type`, `InstanceParsing`, `RdmInstanceArea` → bidirectional coupling |
| `db/property.rs:3-9` | DB query impl imports `api::args::AdminReqParsed` |

### 2a. The `Args` super-struct  [COMPLEX]  `model/src/api/args.rs` (748 LOC)

`Args` (args.rs:209-395) = **~37 optional fields**, deserialized then `Args::init(mode)` → `ArgsUnwrapped` (defaults applied). Fields cluster naturally into groups (this is the breakdown target):

- **input/area:** `area`, `data_points`, `clusters`, `instance`, `parent`
- **clustering:** `cluster_mode`, `radius`, `min_points`, `cluster_split_level`, `max_clusters`, `clustering_args`, `center_clusters`, `genetic_post_processing`
- **routing:** `sort_by`, `route_split_level`, `routing_args`
- **bootstrap:** `calculation_mode`, `s2_level`, `s2_size`, `bootstrapping_args`
- **output:** `return_type`, `geometry_type`, `simplify`, `benchmark_mode`
- **persistence:** `save_to_db`, `save_to_golbat`, `save_to_golbat_only`
- **filters:** `last_seen`, `tth`, `mode`
- **dev:** `dev: DevArgs { bypass_adaptive_partition }`
- **`[DEPRECATED]`** (6): `devices`, `fast`, `generations`, `only_unique`, `route_chunk_size`, `routing_time`

Validation is deferred entirely to `Args::init()` (args.rs:483-662); no route-layer validation.

---

## 3. `algorithms` crate — entry points  [HOT]

All **synchronous + `rayon`** (no async anywhere). Entry fns take **long primitive arg lists**, not structs:

| Fn | file:line | Signature shape |
|---|---|---|
| `clustering::main` | `clustering/mod.rs:23` | **15 params** `(&SingleVec, ClusterMode, f64, usize, &mut Stats, u64, usize, CalculationMode, u8, u8, FeatureCollection, &str, bool, bool, bool) → SingleVec` |
| `routing::main` | `routing/mod.rs:16` | 7 params `(&SingleVec, SingleVec, &SortBy, u64, f64, &mut Stats, &str) → SingleVec` |
| `bootstrap::main` | `bootstrap/mod.rs:15` | 10 params `(FeatureCollection, CalculationMode, Precision, SortBy, u8, u8, u64, &mut Stats, &str, &str) → Vec<Feature>` |

Dispatch: clustering → `Greedy::run` (builder, greedy.rs:85) `[HOT]` 26 commits / `fastest::main` / `s2::cluster` / `Plugin`. Routing → 5 `Sort*` strategies / `Plugin`. `vrp.rs` disabled.

`Stats` (`crate::stats`) is threaded `&mut` through every call — captures cluster/route/total timings (matches the `Stats` returned to Dragonite).

**This is where the job queue wraps:** today the actix handler calls these sync fns inline (blocks the worker). V2 = enqueue job → `spawn_blocking` rayon work → push result.

### 3a. Plugin system  `algorithms/src/plugin.rs` (302 LOC)
- **No trait.** `Plugin` struct (plugin.rs:29) = path + interpreter + args. `Folder` enum {Routing, Clustering, Bootstrap}.
- **Discovery:** filesystem scan of `algorithms/src/{folder}/plugins/{name}`; interpreter inferred by extension (`.py`→python3, `.js`→node, `.sh`→bash, `.ts`→ts-node, else binary).
- **Invocation:** spawns **external process** (`std::process::Command`), pipes stringified points to stdin, parses `lat,lng` lines from stdout, blocks on `child.wait()`.
- Plugins live *inside the crate source tree* — not a config dir, not a manifest. `[COUPLED]`

---

## 4. `api` crate — HTTP surface  [HOT]  (actix-web 4.11)

**61 routes** (35 public `/api/v1`, 26 private `/internal`). State = `web::Data<KojiDb>` (3 DB conns + golbat_type) injected per handler; `nominatim::Client` injected; `SessionMiddleware` (cookie); bearer auth on `/api/v1`, session auth on `/internal`. Bootstrap = `api/src/lib.rs:26` (`start()`).

**Response envelope** `api/src/utils/response.rs:29`: `{ message, status, status_code, data, stats }`.

**Inconsistencies (the "awful APIs"):**
- **GET that mutates:** `GET /api/v1/{geofence,route,project}/push/{id}` → upserts to controller DB + fires Dragonite API call. `[COUPLED]`
- **Envelope not uniform:** `/api/v1/info/` and `/internal/data/*` return raw JSON, no envelope.
- **Path/param naming drift:** `/geofence/area/{geofence}` vs `/route/area/{id}` vs `/data/area/{category}`; kebab (`/save-koji`, `/circle-coverage`) with no consistent rule.
- **Catch-all path segments shadow named routes:** `GET /geofence/{return_type}` and `/geofence/{return_type}/{project}` (same for route) — fragile ordering.
- **Duplicated shapes** across geofence/route/project scopes (`/all`, `/save-koji`, `/push/{id}`, `/{return_type}`) with no shared abstraction.
- **Args overload:** nearly every calc/convert/data endpoint takes the same giant `Args` body and `.init(mode)`-branches internally.

---

## 5. Golbat integration (RDM / Unown / Hybrid)  [RDM removal + Dragonite-API target]

`GolbatType` enum (`model/src/lib.rs:13-18`) = **RDM | Unown | Hybrid**. Auto-detected in `model/src/utils/mod.rs:231-246`: no `CONTROLLER_DB_URL` → RDM; else probe `instance` table → exists ⇒ Hybrid, else ⇒ Unown. `KojiDb` (lib.rs:57) holds 3 conns: `koji`, `golbat` (golbat data), `controller` (dragonite/rdm).

**Reads (golbat data DB) — keep:** `gym/pokestop/spawnpoint/station::Query::{all,area}` (`model/src/db/*.rs`) — SELECT lat/lon (+ despawn_sec / end_time filters).

**Writes to CONTROLLER DB — the "rude writes" to replace with Dragonite API:**
| Path | Site | Target |
|---|---|---|
| Unown/Hybrid | `model/src/db/area.rs:190-227` `upsert_from_geometry` | `area` table cols `pokemon_mode_route` / `fort_mode_route` / `quest_mode_route` / `geofence` |
| RDM | `model/src/db/instance.rs:269-370` | `instance` table `data` JSON |

Call sites (all controller writes): `calculate.rs:117-130`, `calculate.rs:291-305`, `geofence.rs:114-123`, `geofence.rs:157-161`, `route.rs:136-139`, `project.rs:38-47`.

**`GolbatType` branch points (14)** — every one collapses once RDM/Hybrid die and writes move to API: `api/src/utils/{mod.rs:48,request.rs:29}`, `calculate.rs:{103,116,213,290}`, `geofence.rs:{113,156}`, `project.rs:37`, `route.rs:135`, `private/instance.rs:{23,98}`.

**RDM removal checklist:** `model/src/db/mod.rs:95-105,145` (`RdmInstanceArea`, `RdmInstance`, `InstanceParsing::Rdm`); `model/src/api/text.rs:21-31`; `model/src/db/instance.rs:16,200-209`; `model/src/api/mod.rs:2`; `GolbatType::RDM` + `Hybrid` arms everywhere; deprecated env fallbacks `UNOWN_DB_URL`/`UNOWN_DB`/`DATABASE_URL` in `utils/mod.rs:173-191`. The whole `instance` table path + `controller` direct-write path goes away.

---

## 6. External contracts

### 6a. Dragonite → Koji (today, keep stable)  `Dragonite/koji/{main.go,request.go}`
Dragonite POSTs to **`/api/v1/calc/...`** with `KojiArgs` JSON `{instance, radius, min_points, data_points, area, return_type, calculation_mode, s2_level, s2_size, cluster_mode, sort_by}` and expects `KojiResponse{message,data,status,status_code,stats}` where `data = [[lat,lon],...]`. Endpoints used: `calc/bootstrap`, `calc/route/pokestop`, `calc/route/spawnpoint`, `calc/route/fort`, `health`. **Phase-1 constraint: this request/response contract must keep working** (or be versioned) or we break the live Dragonite integration.

### 6b. Koji → Dragonite (target: replace direct DB writes)  `Dragonite/api.md`, `routes/v2_*.go`
Dragonite REST (gin) exposes **Areas**: `GET /areas/`, `GET /areas/:id`, `POST /areas/`, `PATCH /areas/:id`, `DELETE /areas/:id`, `GET /areas/:id/{enable,disable}`, `GET /recalculate/:area_id/{quest,fort,pokemon}`, plus **v2** routes (`v2_areas.go`, `v2_geofence.go`, `v2_geofence_patch.go`, `v2_envelope.go`). Koji should call `PATCH /areas/:id` (route/geofence fields) instead of writing the `area` table. Auth = bearer + user-agent.

### 6c. PR #558 — per-mode geofence overrides  (OPEN, stacked on #557)
Adds `area.{pokemon_mode_fence, quest_mode_fence, fort_mode_fence}` (each: legacy string | GeoJSON Geometry | Feature; NULL ⇒ fall back to base `geofence`). REST: each mode struct gains optional `geofence: ApiGeofence`. #557 makes Dragonite store **canonical GeoJSON Features** + adds `routes.ApiGeofence`, `db.ParseGeofence`, `geo.Geofence` (orb-backed).
**Koji must:** (1) model per-mode fence overrides instead of one fence per area; (2) speak canonical GeoJSON Features over the v2 area API; (3) when recalculating a mode, cluster within that mode's fence (override-or-base). Koji is already GeoJSON-native (geojson crate) — good alignment.

---

## 7. Client touch-points (note only, don't edit phase 1)
- Client consumes the `/internal/admin/*` CRUD + `/internal/routes/*` + `/api/v1/geofence|route|s2|convert` endpoints and the `Response` envelope shape. Any path/verb/envelope change = client change. Inventory the exact calls before phase 2 API redesign so we know the blast radius.

---

## 8. Churn / risk summary
- `[HOT]` clustering/greedy.rs (26), partition.rs (20), calculate.rs (5), s2.rs (6), args.rs (4) — active areas; touch carefully, they have momentum.
- `[COMPLEX]` model crate (3 concerns), args.rs (748 LOC / 37 fields), greedy.rs (868 LOC).
- `[COUPLED]` db↔api in model; GET-mutation in api; plugins-in-source-tree.
- No tests reference most modules `[UNTESTED]` — verify before large moves.
