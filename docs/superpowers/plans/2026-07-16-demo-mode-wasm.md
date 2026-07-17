# Demo Mode + WASM Portfolio Deployment — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship `apps/web` as a fully client-side demo SPA on GitHub Pages, with Koji's clustering/routing/bootstrap algorithms running in-browser via WASM behind a build-time-selected `@api` adapter.

**Architecture:** A shared `koji-calc-api` crate gives the server and `koji-wasm` identical calc request semantics. The web client gains an `@api` module boundary; vite `--mode demo` aliases it to a demo implementation (IndexedDB CRUD + synthetic NYC world + wasm calc worker + in-memory realtime bus). Live builds are behavior-identical to today.

**Tech Stack:** Rust (wasm-pack nightly, wasm-bindgen-rayon), Vite 8 + React 19 + ra-core 5.14, IndexedDB, GitHub Actions + Pages.

**Spec:** `docs/superpowers/specs/2026-07-16-demo-mode-wasm-design.md` (read it first).

## Global Constraints

- Package manager: **bun** (`bun install`, `bun run <script>`). Lockfile is text `bun.lock` — commit it when deps change.
- Branch: `claude/v2`. Commit per task (conventional style). Commit freely without asking (project rule).
- **NEVER name any closed-source reference project in a committed artifact** — code, comments, commits, docs. Describe borrowed patterns generically (e.g. "build-time adapter selection via a vite alias"), with zero attribution.
- Rust: server crate's package name is `koji` (NOT `koji-server`); `cargo build -p koji`. Never pipe cargo through `| tail` (masks exit code).
- wasm build needs **nightly** + `wasm32-unknown-unknown` target + `wasm-pack`; ONLY via `crates/koji-wasm/build-wasm.sh` (plain `cargo build --target wasm32-unknown-unknown` is a known false-failure TLS-link trap).
- TypeScript: prefer `interface` over `type` for object shapes. React: non-component exports live in sibling `.ts` files (react-refresh lint).
- Web tests: `bun run test` (unit), `bun run test:browser` (browser, END of task only — ~100s cold boot), `bun run typecheck`, `bun run lint` if present.
- Rust tests: targeted `cargo test -p <crate>`; full `cargo test --workspace` at phase boundaries.
- Fix all red checks before advancing. Background any command > 5s.
- Scratch files → session scratchpad, never the repo. Never delete files you didn't create.

## Verified Ground-Truth Reference (do not re-derive)

- **Wire calc body** (client `buildCalcBody`, `apps/web/src/map/lib/calc-request.ts`): tagged `mode` = `"route"` (UI "cluster"), `"bootstrap"`, `"routeStats"`; camelCase nested groups (`clustering`, `routing`, `bootstrap`, `dataFilter`), plus `category`, `area` (FeatureCollection).
- **Server calc types** (`crates/koji-service/src/requests/`): `CalcRequest` enum `#[serde(tag = "mode", rename_all = "camelCase")]` with `Cluster/Route/Reroute/Bootstrap/RouteStats`; arg-groups `ClusteringArgs/RoutingArgs/BootstrapArgs/DataFilterArgs/OutputArgs/DevArgs`; inputs `GeoInput/DataPointsArg`; `resolve.rs` defaults `DEFAULT_RADIUS=70.0`, `DEFAULT_S2_LEVEL=15`, `DEFAULT_S2_SIZE=9`, `DEFAULT_MIN_POINTS=1`. Imports: algorithms, koji_core, geojson, serde, utoipa ONLY (no DB/actix) — extraction-safe.
- **Compute cores** (`crates/koji-service/src/public/v2/calc.rs`): `run_cluster_route(data_points, area, clustering_config, routing_config, instance) -> (KojiGeometryCollection, Stats)`; `resolve_cluster_route(req, is_route, data_points, area)`; `run_bootstrap(area, bootstrap_config, routing_config, instance) -> Result<(KojiGeometryCollection, Stats), _>`; `effective_sort(is_route, sort_by)` (route + Unset → Tsp); `centers_collection`.
- **Algorithms signatures:** `clustering::main(&SingleVec, &ClusteringConfig, FeatureCollection, &mut Stats) -> SingleVec`; `routing::main(&SingleVec, SingleVec, Precision, &RoutingConfig, &mut Stats) -> SingleVec`; `bootstrap::main(FeatureCollection, &BootstrapConfig, &RoutingConfig, &mut Stats) -> Vec<Feature>` — **currently `#[cfg(feature = "native")]`-gated** (Task 2 ungates).
- **Options lists:** `clustering::all_clustering_options()`, `routing::all_routing_options()`, `bootstrap::all_bootstrap_options()` in the algorithms crate (plugins lists are native-gated → empty on wasm, which is correct for demo).
- **JobRecord wire:** `{ id, kind, status: "queued"|"running"|"succeeded"|"failed"|"canceled", progress: f32, phase: string|null, result: {data, stats}|null, error: string|null }`. Client `useCalc` (`apps/web/src/components/deck/use-calc.ts`) gets progress/terminal via realtime topic `jobs/{id}` + one-shot safety-net `getJob` — **no polling loop**; demo transport must be a real in-memory bus.
- **Realtime interfaces** (`apps/web/src/components/realtime/types.ts:80-93`): `RealtimeTransport { subscribe, publish, connect?, disconnect?, onReconnect?, onStatusChange? }`. Outbound event frame: `{ topic, type, payload?, meta? }`.
- **Envelope:** demo adapter works at provider level — envelopes (`{status:"ok",data,meta}`) exist only inside `api/live/*`; demo functions return unwrapped data.
- **List row shapes:** `GeofenceRow { id, name, mode, parent, geo_type, projects: number[], property_count }`; `RouteRow { id, name, description, mode, geofence_id, points }`. List query params: `page, per_page, sortBy, order` + filters (geofence: `q, project, parent, geotype, mode`; route: `q, mode, geofenceid, pointsmin, pointsmax`).
- **getOne geo resources:** server returns a GeoJSON Feature (or FC); client `featureToRecord` flattens `properties` + `geometry` + `geo_type`.
- **s2:** `POST /api/v2/s2/{level}` body = `koji_core::BoundsArg` (flattened bbox + `last_seen`/`ids`/`tth`); response data = `koji_core::s2::get_cells(level, min_lat, min_lon, max_lat, max_lon) -> Vec<S2Response>` (`{id, coords: [[lat,lon]]}`); wasm-safe.
- **Convert:** `POST /internal/geometry/convert` = `ConvertReq { area, output, simplify? }` → `KojiGeometryCollection::try_from(FeatureCollection)` → FC out; wasm-safe (pure koji-core).
- **Import:** `ImportRequest { dry_run, items: ImportItemDto[] }` → `{ committed, summary{create,update,skip,fail}, results[{index,name,action,id,reason}] }`.
- **Webhook test result:** `{ delivered, upstream_status, error }`.
- **Config:** `ConfigResponse { start_lat, start_lon, tile_server, logged_in, dangerous, route_plugins, clustering_plugins, bootstrap_plugins }`.
- **Seed asset (already committed):** `apps/web/src/api/demo/seeds/nyc-areas.geo.json` — FC of 32 Manhattan neighborhood Polygons, properties `{ name, mode: "auto_quest", parent: "NY" }`. Remap modes at seed time.
- **koji-wasm today:** exports `cluster(ClusterRequest) -> ClusterResponse` (tsify DTOs), `version()`, `init_thread_pool`; `build-wasm.sh` patches the rayon worker helper import; `pkg/` = ES module + 1.8MB wasm.
- **Vitest:** projects `unit` (jsdom + MSW, excludes `*.browser.test.*`) and `browser` (Playwright, only `*.browser.test.*`).
- **Aliases:** `@/*` → `./src/*` in tsconfig.app.json + vite; `shadmin-core` → ra-core.

---

### Task 1: Extract `koji-calc-api` shared crate (request types + compute core)

**Files:**
- Create: `crates/koji-calc-api/Cargo.toml`, `crates/koji-calc-api/src/lib.rs`, `crates/koji-calc-api/src/compute.rs`
- Move: `crates/koji-service/src/requests/{ops.rs,inputs.rs,groups.rs,resolve.rs,config.rs}` → `crates/koji-calc-api/src/`
- Modify: `crates/koji-service/src/requests/mod.rs` (becomes re-export), `crates/koji-service/src/public/v2/calc.rs` (compute fns move out), root `Cargo.toml` (workspace member + dep), `crates/koji-service/Cargo.toml`
- Test: existing server tests (moved along), plus new `crates/koji-calc-api/src/compute.rs` unit tests

**Interfaces:**
- Produces crate `koji-calc-api` with: everything `koji-service::requests` exported today (`CalcRequest`, `CalcJobRequest`, `ClusterReq`, `RerouteReq`, `BootstrapReq`, `StatsReq`, `ConvertReq`, `SimplifyReq`, `MergePointsReq`, `GeoInput`, `DataPointsArg`, arg-groups, configs, `ReturnTypeArg`) — visibility raised `pub(crate)` → `pub` where needed.
- Produces `koji_calc_api::compute` with the pure compute cores moved verbatim from `calc.rs`: `pub fn run_cluster_route(...)`, `pub fn resolve_cluster_route(...)`, `pub fn run_bootstrap(...) -> Result<(KojiGeometryCollection, Stats), String>` (error type changes from `JobError` to `String`; the server maps it back), `pub fn effective_sort(...)`, `pub fn centers_collection(...)`.
- `resolve.rs` constants/fns become `pub` (`DEFAULT_RADIUS`, `resolve_data_points`, etc.).
- utoipa `ToSchema` derives stay but behind a default-on feature `schema` (`#[cfg_attr(feature = "schema", derive(ToSchema))]`), so koji-wasm can depend with `default-features = false`.

- [ ] **Step 1:** Create the crate:

```toml
# crates/koji-calc-api/Cargo.toml
[package]
name = "koji-calc-api"
version = "0.1.0"
edition = "2024"
publish = false

[features]
default = ["schema"]
schema = ["dep:utoipa"]

[dependencies]
algorithms = { workspace = true }
koji-core  = { workspace = true }
geojson    = { workspace = true }
serde      = { workspace = true }
serde_json = { workspace = true }
utoipa     = { workspace = true, optional = true }
```

Add `"crates/koji-calc-api"` to root `Cargo.toml` `[workspace] members` (enumerated list, not glob) and `koji-calc-api = { path = "crates/koji-calc-api" }` to `[workspace.dependencies]`.

- [ ] **Step 2:** `git mv` the five module files into `crates/koji-calc-api/src/`; write `lib.rs` declaring `pub mod ops; pub mod inputs; pub mod groups; pub mod resolve; pub mod config; pub mod compute;` plus `pub use` re-exports mirroring the old `requests/mod.rs` surface. Convert every `#[derive(... ToSchema ...)]` to split derives with `#[cfg_attr(feature = "schema", derive(utoipa::ToSchema))]`. Raise `pub(crate)` → `pub` on items the server or wasm consumes.
- [ ] **Step 3:** Move the five pure fns from `crates/koji-service/src/public/v2/calc.rs` into `crates/koji-calc-api/src/compute.rs` verbatim, changing only: `JobError::internal(...)` → `String` error (`run_bootstrap` returns `Result<_, String>`). Server's `calc.rs` keeps `CalculateHandler`/`CalcPayload`/dispatch and calls `koji_calc_api::compute::*`, mapping `String` → `JobError::internal`.
- [ ] **Step 4:** Replace `crates/koji-service/src/requests/mod.rs` body with `pub(crate) use koji_calc_api::*;` (keep the module so existing `crate::requests::` paths compile). Add `koji-calc-api = { workspace = true }` to `crates/koji-service/Cargo.toml`.
- [ ] **Step 5:** Move `#[cfg(test)]` tests that lived in the moved files along with them. Add one new compute test:

```rust
// in compute.rs #[cfg(test)]
#[test]
fn cluster_route_produces_centers_and_stats() {
    use algorithms::clustering::{CalculationMode, ClusterMode, ClusteringConfig, S2Config};
    use algorithms::routing::{RoutingConfig, SortBy};
    let pts: koji_core::SingleVec = vec![[40.0, -74.0], [40.0001, -74.0], [40.0002, -74.0]];
    let cfg = ClusteringConfig {
        mode: ClusterMode::Fastest, radius: 200.0, min_points: 1, max_clusters: usize::MAX,
        calculation_mode: CalculationMode::Radius, s2: S2Config::default(),
        center_clusters: false, plugin_args: String::new(),
    };
    let routing = RoutingConfig { sort_by: SortBy::Random, plugin_args: String::new() };
    let empty = geojson::FeatureCollection { bbox: None, features: vec![], foreign_members: None };
    let (coll, stats) = run_cluster_route(&pts, empty, &cfg, &routing, "t");
    assert!(stats.total_clusters >= 1);
    assert!(!coll.geometries.is_empty());
}
```

(If `KojiGeometryCollection`'s field is not `geometries`, read `crates/koji-core` and assert on whatever the collection exposes — non-empty is the point.)

- [ ] **Step 6:** Run: `cargo test -p koji-calc-api -p koji 2>&1` (background). Expected: all green, server behavior unchanged. Also `cargo fmt --all` and `cargo clippy -p koji-calc-api -p koji` clean.
- [ ] **Step 7:** Commit: `refactor(calc): extract koji-calc-api shared crate (request types + compute core)`

---

### Task 2: Ungate bootstrap for wasm

**Files:**
- Modify: `crates/algorithms/src/bootstrap/mod.rs` (lines 1-2 gated `std::time::Instant` import; line ~24 gated `main`)

**Interfaces:**
- Produces: `bootstrap::main` compiled without the `native` feature (same signature).

- [ ] **Step 1:** In `bootstrap/mod.rs`, replace the `#[cfg(feature = "native")] use std::time::Instant;` with `use web_time::Instant;` (matching `bootstrap/radius.rs` and `bootstrap/s2.rs`), and remove the `#[cfg(feature = "native")]` gate from `pub fn main`. If `main`'s body references anything else native-only (grep the fn body for `sysinfo`, `koji_plugins`, `std::time`), keep those inner blocks gated and give them non-native fallbacks consistent with how `clustering/greedy.rs:281-287` does it.
- [ ] **Step 2:** Verify both worlds: `cargo test -p algorithms` (native) AND `cargo check -p algorithms --no-default-features --target wasm32-unknown-unknown` (needs the target installed; `check` doesn't link so the TLS trap doesn't apply). Expected: both green.
- [ ] **Step 3:** Commit: `feat(algorithms): compile bootstrap on wasm (web_time, ungate main)`

---

### Task 3: koji-wasm calc-parity exports

**Files:**
- Modify: `crates/koji-wasm/Cargo.toml` (add `koji-calc-api` default-features=false), `crates/koji-wasm/src/lib.rs`
- Create: `crates/koji-wasm/src/calc.rs`
- Keep: existing `cluster()`/dto/convert untouched.

**Interfaces:**
- Consumes: `koji_calc_api::{CalcJobRequest, CalcRequest, compute::*, resolve::*}`, `koji_core::s2::get_cells`, `KojiGeometryCollection::try_from`.
- Produces wasm exports (all `JsValue`-based via `serde_wasm_bindgen`):
  - `calc(req: JsValue) -> Result<JsValue, JsError>` — input: the exact `POST /api/v2/jobs` body (client `buildCalcBody` output, with `dataPoints`/`clusters` injected by the JS caller where the server would have resolved them from the DB); output: `{ data: <geojson FC>, stats: <full Stats> }`.
  - `algorithm_options() -> JsValue` — `{ clustering: string[], routing: string[], bootstrap: string[] }` from `all_*_options()`.
  - `s2_cells(level: u8, min_lat: f64, min_lon: f64, max_lat: f64, max_lon: f64) -> JsValue` — `Vec<S2Response>`.
  - `convert_geometry(area: JsValue) -> Result<JsValue, JsError>` — FC in → normalized FC out via `KojiGeometryCollection`.

- [ ] **Step 1:** Write failing native tests in `calc.rs` `#[cfg(test)]` (test the core fns, not the `JsValue` wrappers — mirror how `lib.rs` tests `cluster` today by testing an inner `calc_impl(serde_json::Value) -> Result<serde_json::Value, String>`):

```rust
#[test]
fn calc_route_mode_end_to_end() {
    let body = serde_json::json!({
        "mode": "route",
        "category": "spawnpoint",
        "area": { "type": "FeatureCollection", "features": [] },
        "dataPoints": [[40.0, -74.0], [40.0001, -74.0], [40.0002, -74.0]],
        "clustering": { "calculationMode": "radius", "radius": 200, "minPoints": 1 },
        "routing": { "sortBy": "random" }
    });
    let out = calc_impl(body).expect("calc should succeed");
    assert_eq!(out["data"]["type"], "FeatureCollection");
    assert!(out["stats"]["total_clusters"].as_u64().unwrap() >= 1);
}

#[test]
fn calc_bootstrap_mode_end_to_end() {
    // A small square around lower Manhattan; radius bootstrap.
    let body = serde_json::json!({
        "mode": "bootstrap",
        "area": { "type": "FeatureCollection", "features": [{
            "type": "Feature", "properties": {},
            "geometry": { "type": "Polygon", "coordinates": [[
                [-74.02, 40.70], [-74.00, 40.70], [-74.00, 40.72], [-74.02, 40.72], [-74.02, 40.70]
            ]]}
        }]},
        "bootstrap": { "calculationMode": "radius", "radius": 300 }
    });
    let out = calc_impl(body).expect("bootstrap should succeed");
    assert_eq!(out["data"]["type"], "FeatureCollection");
    assert!(!out["data"]["features"].as_array().unwrap().is_empty());
}

#[test]
fn calc_route_stats_mode() {
    let body = serde_json::json!({
        "mode": "routeStats",
        "dataPoints": [[40.0, -74.0], [40.0001, -74.0]],
        "clusters": [[40.00005, -74.0]],
        "radius": 100, "minPoints": 1
    });
    let out = calc_impl(body).expect("routeStats should succeed");
    assert!(out["stats"]["total_points"].as_u64().unwrap() == 2);
}

#[test]
fn options_match_algorithms_crate() {
    let o = options_impl();
    assert!(o["clustering"].as_array().unwrap().iter().any(|v| v == "balanced"));
    assert!(o["routing"].as_array().unwrap().iter().any(|v| v == "tsp"));
    assert!(o["bootstrap"].as_array().unwrap().iter().any(|v| v == "radius"));
}
```

- [ ] **Step 2:** `cargo test -p koji-wasm` → FAIL (fns missing).
- [ ] **Step 3:** Implement `calc.rs`: `calc_impl` decodes `CalcJobRequest` from the JSON value, then mirrors the server dispatch in `crates/koji-service/src/public/v2/calc.rs::run` — `Cluster/Route` → `resolve_cluster_route` (+ `is_route`), `Bootstrap` → resolve args → `compute::run_bootstrap`, `Reroute`/`RouteStats` → the same paths `calc.rs` uses (read that dispatch and reproduce it; area resolution: `GeoInput` → `FeatureCollection`, `resolve_data_points` for points/clusters). Serialize result exactly like the server: `json!({ "data": FeatureCollection::from(collection), "stats": stats })` (copy the exact outbound conversion the server uses at its wire boundary). Thin `#[wasm_bindgen]` wrappers do `serde_wasm_bindgen` in/out. `options_impl` returns the three `all_*_options()` arrays.
- [ ] **Step 4:** `cargo test -p koji-wasm` → PASS. Then real wasm build: `bash crates/koji-wasm/build-wasm.sh` (background; needs `rustup toolchain install nightly`, `rustup target add wasm32-unknown-unknown --toolchain nightly`, `cargo install wasm-pack` if missing). Expected: `pkg/` regenerated, no errors.
- [ ] **Step 5:** Commit: `feat(wasm): calc-parity exports (calc, algorithm_options, s2_cells, convert_geometry)`

---

### Task 4: `@api` boundary — types + live move (behavior-neutral)

**Files:**
- Create: `apps/web/src/api/types.ts`, `apps/web/src/api/index.live.ts`, `apps/web/src/api/live/{data-provider.ts,auth-provider.ts,calc.ts,markers.ts,s2.ts,geo-features.ts,config.ts,import.ts,publish.ts,webhook-test.ts,realtime.ts}`
- Modify (imports only / fetch-extraction): `apps/web/src/data-provider.ts`, `apps/web/src/auth-provider.ts` (becomes re-export), `apps/web/src/map/data/calc-client.ts` (delete; consumers import `@api`), `apps/web/src/map/data/use-markers.ts`, `apps/web/src/map/data/use-s2-cells.ts`, `apps/web/src/map/data/use-geo-features.ts`, `apps/web/src/lib/use-start-center.ts`, `apps/web/src/lib/import-api.ts` (delete; consumers import `@api`), `apps/web/src/components/actions/publish-button.tsx`, `apps/web/src/resources/webhook/webhook-test-button.tsx`, `apps/web/src/components/deck/use-calc.ts` (import path only)
- Modify: `apps/web/vite.config.ts`, `apps/web/tsconfig.app.json` (add `@api` alias → `./src/api/index.live.ts` for now)
- Test: existing suites (this task is behavior-neutral); `apps/web/src/api/api-surface.test.ts`

**Interfaces:**
- Produces `@api` named exports (both modes must implement this exact surface; `types.ts` declares it):

```ts
// apps/web/src/api/types.ts
import type { AuthProvider, DataProvider } from "ra-core";
import type { RealtimeTransport } from "@/components/realtime";

export interface JobRecord {
  id: number | string;
  status: string;
  progress: number;
  phase: string | null;
  result?: { data?: unknown; stats?: unknown } | null;
  error?: string | null;
}
export interface AlgorithmOptions { clustering: string[]; routing: string[]; bootstrap: string[] }
export interface ConfigResponse {
  start_lat: number; start_lon: number; tile_server: string;
  logged_in: boolean; dangerous: boolean;
  route_plugins: string[]; clustering_plugins: string[]; bootstrap_plugins: string[];
}
export interface PublishResult { ok: boolean; warning: boolean; message: string }
export interface WebhookTestResult { delivered: boolean; upstream_status: number | null; error: string | null }

/** The full adapter surface. index.live.ts and index.demo.ts must both satisfy it. */
export interface ApiSurface {
  baseDataProvider: DataProvider;
  authProvider: AuthProvider;
  createRealtimeTransport: () => RealtimeTransport;
  loadConfig: () => Promise<ConfigResponse>;
  submitCalc: (body: Record<string, unknown>) => Promise<string>;
  getJob: (id: string) => Promise<JobRecord>;
  getAlgorithms: () => Promise<AlgorithmOptions>;
  // Keep the EXACT current signatures for the four below — copy them from the
  // existing modules when moving (they are consumed by hooks that don't change):
  fetchMarkers: typeof import("./live/markers").fetchMarkers;
  fetchS2Cells: typeof import("./live/s2").fetchS2Cells;
  fetchFeatureCollection: typeof import("./live/geo-features").fetchFeatureCollection;
  fetchGeofencesByIds: typeof import("./live/geo-features").fetchGeofencesByIds;
  fetchGeofencesByBbox: typeof import("./live/geo-features").fetchGeofencesByBbox;
  postImport: typeof import("./live/import").postImport;
  postConvert: typeof import("./live/import").postConvert;
  publishRecord: (resource: string, id: string | number) => Promise<PublishResult>;
  testWebhook: (id: string | number) => Promise<WebhookTestResult>;
  /** Demo only; no-op in live. */
  resetDemo: () => Promise<void>;
}
```

(If `typeof import(...)` self-reference proves awkward, declare the concrete signatures inline by copying them from the moved files — the point is: **hooks keep their current call signatures**.)

- [ ] **Step 1:** Add alias in both configs: vite `resolve.alias` `"@api": path.resolve(__dirname, "src/api/index.live.ts")`; tsconfig.app.json `"@api": ["./src/api/index.live.ts"]`.
- [ ] **Step 2:** Move code (extraction, not rewrite): raw-fetch functions move into `api/live/*`; UI hooks stay put and import from `@api`.
  - `data-provider.ts`: `RESOURCE_MAP/segFor/itemPath/featureToRecord/toQuery/getListImpl/serializeGeofenceWrite/baseDataProvider` → `api/live/data-provider.ts` (exports `baseDataProvider`, `serializeGeofenceWrite`). Root `src/data-provider.ts` becomes the mode-agnostic composition:

```ts
import { baseDataProvider, createRealtimeTransport } from "@api";
import { realtimeDataProvider, inMemoryLockProvider } from "@/components/realtime";
export { serializeGeofenceWrite } from "@api"; // if any consumer imports it from here today — check and preserve
export const dataProvider = realtimeDataProvider(
  baseDataProvider,
  createRealtimeTransport(),
  { locks: inMemoryLockProvider() },
);
```

  - `api/live/realtime.ts`: `export const createRealtimeTransport = () => webSocketTransport({ url: "/internal/realtime" });`
  - `auth-provider.ts` → `api/live/auth-provider.ts`; old path re-exports from `@api` (or update `App.tsx` import — prefer updating the import).
  - `map/data/calc-client.ts` → `api/live/calc.ts`; update `use-calc.ts` + any other consumer (`grep -rn "calc-client" apps/web/src`) to `import { getJob, submitCalc } from "@api"`. `JobRecord`/`AlgorithmOptions` move to `api/types.ts`; fix type imports (`grep -rn "calc-client\|AlgorithmOptions" apps/web/src`).
  - `use-markers.ts`/`use-s2-cells.ts`/`use-geo-features.ts`: move the raw fetch fns to `api/live/{markers,s2,geo-features}.ts`; hooks keep react-query wrappers importing from `@api`.
  - `lib/use-start-center.ts`: fetch+unwrap moves to `api/live/config.ts` as `loadConfig()`; the hook keeps its module-level cache and derives `[start_lat, start_lon]`.
  - `lib/import-api.ts` fns → `api/live/import.ts`; update consumers (`grep -rn "import-api" apps/web/src`); keep the wire-shape interfaces (`ImportRequest` etc.) exported from `api/types.ts` or a sibling — one home, no duplicates.
  - `publish-button.tsx`: extract the fetch into `api/live/publish.ts`:

```ts
import { internalFetch } from "@/lib/http";
import { segFor } from "./data-provider";
import type { PublishResult } from "../types";

export async function publishRecord(resource: string, id: string | number): Promise<PublishResult> {
  const res = await internalFetch(`/${segFor(resource)}/${id}/publish`, { method: "POST" });
  if (res.status === 422) {
    const body = res.json as { error?: { message?: string } | string } | null;
    const msg = typeof body?.error === "string" ? body.error : body?.error?.message;
    return { ok: false, warning: true, message: msg ?? "Cannot publish: no linked area" };
  }
  if (res.status < 200 || res.status >= 300)
    return { ok: false, warning: false, message: `Publish failed (${res.status})` };
  return { ok: true, warning: false, message: "Published" };
}
```

  (Check the CURRENT 422 body parse in `publish-button.tsx:39-50` and preserve its exact semantics — the snippet above must be reconciled with what the component does today, then the component renders toasts from `PublishResult`.)
  - `webhook-test-button.tsx`: same extraction → `api/live/webhook-test.ts` `testWebhook(id)`.
  - `api/live/*` add `export const resetDemo = async (): Promise<void> => {};`
- [ ] **Step 3:** `api/index.live.ts` re-exports everything; add a conformance check so both index files stay honest:

```ts
// apps/web/src/api/api-surface.test.ts
import { describe, expect, it } from "vitest";
import type { ApiSurface } from "./types";
import * as live from "./index.live";

describe("@api surface", () => {
  it("live implements ApiSurface", () => {
    const surface: ApiSurface = live; // compile-time check
    expect(surface.baseDataProvider.getList).toBeTypeOf("function");
    expect(surface.submitCalc).toBeTypeOf("function");
    expect(surface.resetDemo).toBeTypeOf("function");
  });
});
```

- [ ] **Step 4:** Sweep for leftovers: `grep -rn "internalFetch\|apiV2Fetch" apps/web/src --include="*.ts*" | grep -v "src/api/live\|src/lib/http"` → only `check-for-application-update.tsx` (its `index.html` poll is allowed to stay) and `lib/http.ts` itself may remain.
- [ ] **Step 5:** Run `bun run test` + `bun run typecheck` (parallel, background) → green. Then `bun run test:browser` (background) → green.
- [ ] **Step 6:** Commit: `refactor(web): route all network surfaces through the @api boundary (live)`

---

### Task 5: Vite demo-mode wiring + scripts + demo stub

**Files:**
- Modify: `apps/web/vite.config.ts`, `apps/web/package.json`, `apps/web/tsconfig.app.json`, `apps/web/src/vite-env.d.ts` (declare `__DEMO__`)
- Create: `apps/web/src/api/index.demo.ts` (stub), `apps/web/src/api/demo/wasm-pkg.d.ts` (if needed for pkg types)

**Interfaces:**
- Produces: `vite --mode demo` builds/serves with `@api` → `index.demo.ts`, `__DEMO__ === true`, `base: "/Koji/"`; wasm pkg importable as `@koji-wasm` (alias → `../../crates/koji-wasm/pkg`).
- `package.json` scripts: `"dev:demo": "vite --mode demo --port 5273"`, `"build:demo": "tsc -b && vite build --mode demo"`, `"wasm:build": "bash ../../crates/koji-wasm/build-wasm.sh"`.

- [ ] **Step 1:** vite.config.ts — make the export a function of mode:

```ts
export default defineConfig(({ mode }) => {
  const demo = mode === "demo";
  return {
    base: demo ? "/Koji/" : "/",
    define: { __DEMO__: JSON.stringify(demo) },
    resolve: {
      alias: {
        "@api": path.resolve(__dirname, demo ? "src/api/index.demo.ts" : "src/api/index.live.ts"),
        "@koji-wasm": path.resolve(__dirname, "../../crates/koji-wasm/pkg"),
        // ...existing aliases unchanged
      },
      // ...existing dedupe
    },
    // demo dev needs cross-origin isolation for SharedArrayBuffer (rayon);
    // credentialless keeps cross-origin basemap tiles loadable.
    server: {
      ...(demo
        ? { headers: { "Cross-Origin-Opener-Policy": "same-origin", "Cross-Origin-Embedder-Policy": "credentialless" } }
        : { proxy: /* existing proxy config unchanged */ }),
    },
    optimizeDeps: { exclude: ["@koji-wasm"] },
    worker: { format: "es" as const },
    // ...rest unchanged (preview proxy stays live-only)
  };
});
```

(Merge carefully with the existing config — proxy/dedupe/preview blocks must survive for live mode. `__DEMO__` also goes into tsconfig types via `declare const __DEMO__: boolean;` in `vite-env.d.ts`.)

- [ ] **Step 2:** `index.demo.ts` stub: re-export live's types-compatible surface where every function throws `new Error("demo: not implemented yet")` except `resetDemo` (no-op) — a plain object satisfying `ApiSurface` so `tsc` enforces conformance from day one. Extend `api-surface.test.ts` with a demo-conformance compile check (import `* as demo from "./index.demo"`, assign to `ApiSurface`).
- [ ] **Step 3:** Verify: `bun run test && bun run typecheck` green; `bun run build` (live) green; `bun run build:demo` green (stub demo bundle). Check live build output does NOT contain the string `"demo: not implemented"` (`grep -r "not implemented yet" apps/web/dist` → empty) — proves tree-shaking/alias isolation.
- [ ] **Step 4:** Commit: `feat(web): vite demo mode (@api alias, __DEMO__, base, wasm pkg alias)`

---

### Task 6: Demo world — idb, seeds, synthetic markers

**Files:**
- Create: `apps/web/src/api/demo/db.ts`, `apps/web/src/api/demo/seeds/seed.ts`, `apps/web/src/api/demo/seeds/markers.ts`, `apps/web/src/api/demo/seeds/prng.ts`, `apps/web/src/api/demo/seeds/geometry.ts`
- Test: `apps/web/src/api/demo/seeds/prng.test.ts`, `apps/web/src/api/demo/seeds/markers.test.ts`, `apps/web/src/api/demo/seeds/seed.test.ts`

**Interfaces:**
- `db.ts` produces: `SEED_VERSION` (number, bump to force reseed), `openDemoDb(): Promise<IDBDatabase>` (creates stores `geofences,routes,projects,properties,webhooks,tileservers,meta` keyPath `id` except `meta` key `k`), `tx<T>(store, mode, fn)` promise helper, `allRows(store)`, `putRow(store, row)`, `deleteRow(store, id)`, `nextId(store)`. Hand-rolled (~80 lines) — **no new dependency**. If IndexedDB is unavailable, export an in-memory Map-backed fallback with the same functions + `persistent: false` flag (a toast consumes it later).
- `prng.ts` produces: `mulberry32(seed: number): () => number` (deterministic PRNG).
- `geometry.ts` produces: `pointInPolygon([lat, lon], polygonCoords): boolean` (ray cast, geojson [lon,lat] ring input), `polygonBbox(coords): {minLat,minLon,maxLat,maxLon}`, `featureBbox(feature)`.
- `markers.ts` produces: `generateMarkers(fc: FeatureCollection): MarkerStore` where `MarkerStore = { query(category: string, opts: { bbox?: Bbox; areaFeatures?: Feature[]; lastSeen?: number; tth?: "All"|"Known"|"Unknown" }): [number, number][] }`. Deterministic: seed derives from feature index + category. Densities per category: spawnpoint ~600/polygon (clustered: 6-12 gaussian kernels per polygon + 20% uniform scatter), pokestop ~120, gym ~40, station ~15. Spawnpoints carry synthetic `updatedAt` (spread over last 30 days from a FIXED epoch constant, not `Date.now()`, so tests are stable) and `tthKnown: boolean` (~60% true); `lastSeen`/`tth` filters mirror server semantics (`updatedAt >= lastSeen`; tth Known/Unknown filter applies to spawnpoints only).
- `seed.ts` produces: `ensureSeeded(): Promise<void>` — checks `meta.seedVersion`; on mismatch wipes stores and reseeds: 32 geofence rows from `nyc-areas.geo.json` (id = index+1; `mode` remapped deterministically `["pokemon","fort","quest"][i % 3]`; `geo_type: "Polygon"`; bbox fields; `projects` membership; `properties: []`; `parent: null`; fixed `created_at`/`updated_at` ISO strings), 3 projects (`Downtown` lat < 40.73, `Midtown` 40.73–40.78, `Uptown` > 40.78 by polygon centroid; ids 1-3), 4 properties (one per a few categories with `default_value`), 1 inactive webhook (`url: "https://example.com/hook"`, topics `["geofence.updated"]`), 2 tile servers (`Carto Light`: `https://basemaps.cartocdn.com/gl/positron-gl-style/style.json` — check what shape `tile_server` rows hold today by reading a live row usage/`TILE_SERVER` env; store the same shape). Also exports `resetDemoWorld(): Promise<void>` (wipe + reseed). Route seeding happens in Task 8 (needs wasm).

- [ ] **Step 1:** Write failing tests first:

```ts
// prng.test.ts
import { describe, expect, it } from "vitest";
import { mulberry32 } from "./prng";
describe("mulberry32", () => {
  it("is deterministic per seed", () => {
    const a = mulberry32(42), b = mulberry32(42);
    expect([a(), a(), a()]).toEqual([b(), b(), b()]);
  });
  it("stays in [0,1)", () => {
    const r = mulberry32(7);
    for (let i = 0; i < 1000; i++) { const v = r(); expect(v).toBeGreaterThanOrEqual(0); expect(v).toBeLessThan(1); }
  });
});
```

```ts
// markers.test.ts
import { describe, expect, it } from "vitest";
import nyc from "./nyc-areas.geo.json";
import { generateMarkers } from "./markers";
import { pointInPolygon } from "./geometry";

const fc = nyc as GeoJSON.FeatureCollection;
describe("generateMarkers", () => {
  it("is deterministic", () => {
    const a = generateMarkers(fc).query("spawnpoint", {});
    const b = generateMarkers(fc).query("spawnpoint", {});
    expect(a).toEqual(b);
  });
  it("generates all four categories with sane counts", () => {
    const store = generateMarkers(fc);
    const sp = store.query("spawnpoint", {});
    expect(sp.length).toBeGreaterThan(5000);
    expect(store.query("gym", {}).length).toBeGreaterThan(300);
    expect(store.query("pokestop", {}).length).toBeGreaterThan(1000);
    expect(store.query("station", {}).length).toBeGreaterThan(100);
  });
  it("all points fall inside their source polygons' union bbox", () => {
    const store = generateMarkers(fc);
    for (const [lat, lon] of store.query("gym", {})) {
      expect(lat).toBeGreaterThan(40.69); expect(lat).toBeLessThan(40.89);
      expect(lon).toBeGreaterThan(-74.03); expect(lon).toBeLessThan(-73.90);
    }
  });
  it("bbox query filters", () => {
    const store = generateMarkers(fc);
    const all = store.query("spawnpoint", {});
    const some = store.query("spawnpoint", { bbox: { minLat: 40.70, minLon: -74.02, maxLat: 40.72, maxLon: -74.00 } });
    expect(some.length).toBeGreaterThan(0);
    expect(some.length).toBeLessThan(all.length);
  });
  it("area query keeps only in-polygon points", () => {
    const store = generateMarkers(fc);
    const feat = fc.features[0];
    const pts = store.query("spawnpoint", { areaFeatures: [feat] });
    const ring = (feat.geometry as GeoJSON.Polygon).coordinates;
    for (const p of pts.slice(0, 50)) expect(pointInPolygon(p, ring)).toBe(true);
  });
  it("tth filter partitions spawnpoints", () => {
    const store = generateMarkers(fc);
    const all = store.query("spawnpoint", {}).length;
    const known = store.query("spawnpoint", { tth: "Known" }).length;
    const unknown = store.query("spawnpoint", { tth: "Unknown" }).length;
    expect(known + unknown).toBe(all);
    expect(known).toBeGreaterThan(0); expect(unknown).toBeGreaterThan(0);
  });
});
```

```ts
// seed.test.ts — runs against the in-memory fallback (jsdom has no real idb;
// force the fallback path explicitly rather than relying on jsdom quirks).
import { describe, expect, it } from "vitest";
import { ensureSeeded, resetDemoWorld } from "./seed";
import { allRows } from "../db";
describe("seed", () => {
  it("seeds 32 geofences + 3 projects and is idempotent", async () => {
    await ensureSeeded();
    await ensureSeeded(); // second call must not duplicate
    const fences = await allRows("geofences");
    expect(fences).toHaveLength(32);
    expect(await allRows("projects")).toHaveLength(3);
    const modes = new Set(fences.map((f: { mode: string }) => f.mode));
    expect(modes).toEqual(new Set(["pokemon", "fort", "quest"]));
    for (const f of fences) expect(f.projects.length).toBeGreaterThan(0);
  });
  it("resetDemoWorld reseeds", async () => {
    await resetDemoWorld();
    expect(await allRows("geofences")).toHaveLength(32);
  });
});
```

- [ ] **Step 2:** `bun run test` → new tests FAIL.
- [ ] **Step 3:** Implement `prng.ts` (mulberry32 one-liner), `geometry.ts` (ray-cast + bbox, note geojson coords are `[lon,lat]` while koji points are `[lat,lon]` — be explicit at every boundary, this is the classic Koji bug), `markers.ts`, `db.ts`, `seed.ts` per the Interfaces block. Gaussian scatter: Box-Muller from two PRNG draws; kernel σ ≈ 0.0015° ; rejection-sample against the polygon.
- [ ] **Step 4:** `bun run test` + `bun run typecheck` → PASS.
- [ ] **Step 5:** Commit: `feat(web): demo world — idb layer, NYC seed, deterministic synthetic markers`

---

### Task 7: Demo endpoints — CRUD provider, auth, config, geo, import, misc

**Files:**
- Create: `apps/web/src/api/demo/{data-provider.ts,auth-provider.ts,config.ts,geo-features.ts,markers.ts,import.ts,misc.ts}`
- Modify: `apps/web/src/api/index.demo.ts` (stub → real exports, calc still stubbed)
- Test: `apps/web/src/api/demo/data-provider.test.ts`, `apps/web/src/api/demo/geo-features.test.ts`, `apps/web/src/api/demo/import.test.ts`

**Interfaces:**
- Consumes: Task 6 (`db.ts`, `seed.ts`, `markers.ts`), `@api` `types.ts`.
- Produces: every `ApiSurface` member except `submitCalc/getJob/getAlgorithms` (Task 8).
- `data-provider.ts`: ra-core `DataProvider` over idb. `getList` mirrors server filters — geofence: `q` (name substring, case-insensitive), `project` (id member), `parent`, `geotype`, `mode`; route: `q`, `mode`, `geofenceid`, `pointsmin`/`pointsmax`; all resources: `sortBy`/`order`/`page`/`per_page`. List returns row shapes (`GeofenceRow`: computed `projects` + `property_count`; `RouteRow`: computed `points` from geometry coordinates count). `getOne` for geo resources returns a GeoJSON Feature (`{ type: "Feature", id, properties: { id, name, mode, parent, projects, properties, ... }, geometry }`) so the existing `featureToRecord` path works unchanged; non-geo return the row. `create/update/delete` write idb, recompute bbox for geofences (`serializeGeofenceWrite` shape accepted), bump `updated_at` from a module-level monotonic counter offset (not `Date.now()` in tests — inject a `now()` fn defaulting to `Date.now`).
- `auth-provider.ts`: every method resolves; `checkAuth` OK; `getPermissions` → `"admin"`.
- `config.ts`: `loadConfig()` → `{ start_lat: 40.758, start_lon: -73.9855, tile_server: <same value shape live uses — read how TILE_SERVER flows to the map and match>, logged_in: true, dangerous: false, route_plugins: [], clustering_plugins: [], bootstrap_plugins: [] }`.
- `geo-features.ts`: `fetchFeatureCollection("geofences"|"routes")` → FC from idb rows (feature per row, properties `{id, name, mode}`); `fetchGeofencesByIds(ids)`; `fetchGeofencesByBbox(bbox, mode?, projects?)` → AABB intersect on stored bbox fields (+ mode/projects filters), mirroring the live signatures exactly.
- `markers.ts`: adapts `MarkerStore.query` to the live `fetchMarkers` signature (area GeoInput or bbox body → opts).
- `import.ts`: `postConvert(features)` → wasm `convert_geometry` (dynamic-import `@koji-wasm`, init once — share the singleton with Task 8's loader; put the wasm module loader in `apps/web/src/api/demo/wasm.ts` now: `export async function getWasm(): Promise<typeof import("@koji-wasm")>` with cached init + thread-pool init guarded by `typeof SharedArrayBuffer !== "undefined"`); `postImport` → dry-run computes summary vs existing names, commit writes rows atomically (single idb tx; name collision per `on_collision`).
- `misc.ts`: `publishRecord` → `{ ok: true, warning: false, message: "Published (demo)" }`; `testWebhook` → `{ delivered: false, upstream_status: null, error: "demo mode — outbound delivery disabled" }`; plugins list (empty) wired into whatever the plugins resource calls on the provider (it's `getList`-driven — verify `RESOURCE_MAP` handling; demo `getList("plugins")` returns `{ data: [], total: 0 }`); `resetDemo` → `resetDemoWorld()` then `location.reload()` (reload lives in the UI button, keep `resetDemo` pure).

- [ ] **Step 1:** Failing tests (representative — keep them wire-shape-accurate):

```ts
// data-provider.test.ts
import { beforeAll, describe, expect, it } from "vitest";
import { demoBaseDataProvider } from "./data-provider";
import { ensureSeeded } from "./seeds/seed";

beforeAll(async () => { await ensureSeeded(); });

describe("demo dataProvider", () => {
  it("lists geofences with pagination + sort + total", async () => {
    const r = await demoBaseDataProvider.getList("geofence", {
      pagination: { page: 1, perPage: 10 },
      sort: { field: "name", order: "ASC" },
      filter: {},
    });
    expect(r.total).toBe(32);
    expect(r.data).toHaveLength(10);
    expect(r.data[0].name <= r.data[1].name).toBe(true);
    expect(r.data[0]).toHaveProperty("projects");
    expect(r.data[0]).toHaveProperty("property_count");
  });
  it("filters by mode and q", async () => {
    const r = await demoBaseDataProvider.getList("geofence", {
      pagination: { page: 1, perPage: 50 },
      sort: { field: "id", order: "ASC" },
      filter: { mode: "quest", q: "park" },
    });
    for (const row of r.data) {
      expect(row.mode).toBe("quest");
      expect(row.name.toLowerCase()).toContain("park");
    }
  });
  it("getOne geofence returns a Feature the admin can flatten", async () => {
    const r = await demoBaseDataProvider.getOne("geofence", { id: 1 });
    // demo returns Feature; live path flattens via featureToRecord — mirror the flattened record here
    expect(r.data.id).toBe(1);
    expect(r.data.geometry?.type).toBe("Polygon");
    expect(r.data.geo_type).toBe("Polygon");
  });
  it("create/update/delete round-trip a route", async () => {
    const created = await demoBaseDataProvider.create("route", {
      data: { name: "t-route", geofence_id: 1, mode: "quest",
        geometry: { type: "MultiPoint", coordinates: [[-74.0, 40.7], [-74.01, 40.71]] } },
    });
    expect(created.data.id).toBeGreaterThan(0);
    const upd = await demoBaseDataProvider.update("route", {
      id: created.data.id, data: { name: "t-route-2" }, previousData: created.data,
    });
    expect(upd.data.name).toBe("t-route-2");
    await demoBaseDataProvider.delete("route", { id: created.data.id });
    const list = await demoBaseDataProvider.getList("route", {
      pagination: { page: 1, perPage: 50 }, sort: { field: "id", order: "ASC" }, filter: { q: "t-route" },
    });
    expect(list.data).toHaveLength(0);
  });
});
```

(NOTE: `getOne` demo shape decision — EITHER return the raw Feature and reuse the shared `featureToRecord` by exporting it from a mode-agnostic module, OR return the flattened record directly. Pick ONE, make the test match, and keep `apps/web/src` consumers unchanged. Recommended: move `featureToRecord` into `apps/web/src/api/shared.ts` and have BOTH providers return flattened records — then delete the flatten step from live's `getOne`? NO — live getOne currently flattens inside the provider; keep live untouched and have demo return the already-flattened record. The `ApiSurface` contract is `DataProvider`, whose `getOne` returns records, so flattened is the contract.)

```ts
// geo-features.test.ts
it("bbox query intersects stored bboxes", async () => {
  await ensureSeeded();
  const fc = await fetchGeofencesByBbox("-74.02,40.70,-74.00,40.72");
  expect(fc.type).toBe("FeatureCollection");
  expect(fc.features.length).toBeGreaterThan(0);
  expect(fc.features.length).toBeLessThan(32);
  for (const f of fc.features) expect(f.properties).toHaveProperty("name");
});
```

(Match the live `fetchGeofencesByBbox` signature exactly — read `api/live/geo-features.ts` after Task 4 and mirror its params/return.)

```ts
// import.test.ts — convert() needs wasm; unit-test the idb import path only,
// with convert mocked. Wasm-backed convert is covered by the browser smoke.
it("dry run reports create vs skip; commit writes", async () => {
  await resetDemoWorld();
  const items = [{ kind: "geofence" as const, name: "Battery Park City", geometry: SQUARE, projects: [], on_collision: "skip" as const },
                 { kind: "geofence" as const, name: "Brand New", geometry: SQUARE, projects: [1], on_collision: "skip" as const }];
  const dry = await postImport({ dry_run: true, items });
  expect(dry.committed).toBe(false);
  expect(dry.summary).toMatchObject({ create: 1, skip: 1 });
  const real = await postImport({ dry_run: false, items });
  expect(real.committed).toBe(true);
  const rows = await allRows("geofences");
  expect(rows.some((r: { name: string }) => r.name === "Brand New")).toBe(true);
});
```

- [ ] **Step 2:** `bun run test` → FAIL. Implement per Interfaces. Sorting: string fields `localeCompare`, numbers numeric; unknown `sortBy` field falls back to `id`.
- [ ] **Step 3:** Wire `index.demo.ts`: real exports for everything except `submitCalc/getJob/getAlgorithms` (still throwing stubs). Demo-conformance compile test from Task 5 still passes.
- [ ] **Step 4:** `bun run test` + `bun run typecheck` → PASS.
- [ ] **Step 5:** Commit: `feat(web): demo endpoints — idb CRUD provider, auth, config, geo, import, misc`

---

### Task 8: Realtime bus, calc worker + job facade, COI, seed routes

**Files:**
- Create: `apps/web/src/api/demo/realtime.ts`, `apps/web/src/api/demo/calc/worker.ts`, `apps/web/src/api/demo/calc/facade.ts`, `apps/web/public/coi-serviceworker.js` (vendored, demo-only registration), `apps/web/src/api/demo/coi.ts`
- Modify: `apps/web/src/api/index.demo.ts` (calc exports + createRealtimeTransport), `apps/web/src/api/demo/seeds/seed.ts` (route seeding step), `apps/web/src/main.tsx` (demo boot: COI register + ensureSeeded gate — coordinate with Task 9's splash)
- Test: `apps/web/src/api/demo/realtime.test.ts`, `apps/web/src/api/demo/calc/facade.test.ts`

**Interfaces:**
- Consumes: `RealtimeTransport` interface, Task 6 marker store, Task 7 `wasm.ts` loader, `JobRecord` type.
- Produces:
  - `realtime.ts`: `demoBus: RealtimeTransport & { emit(topic, event): void }` — in-memory topic map; `publish` resolves immediately and dispatches to local subscribers (queueMicrotask); `onStatusChange` fires `"connected"` immediately; `subscribe` returns unsubscribe. `createRealtimeTransport = () => demoBus`.
  - `facade.ts`: `submitCalc(body)` → `"demo-<n>"` id; resolves `dataPoints` (route/reroute/routeStats need them: from `body.area` + `body.category` + `body.dataFilter` via marker store when `dataPoints` absent) and injects into the body; publishes `jobs/{id}` `{type:"status", payload:{status:"running", progress:0.1, phase:"clustering"}}`; posts to worker; on worker reply stores `JobRecord` and publishes terminal `{status:"succeeded"|"failed"}` event. `getJob(id)` → stored record (throw on unknown id like live 404 → `HttpError`-shaped `Error`). `getAlgorithms()` → worker `options` call, cached.
  - `worker.ts` (module worker): imports `@koji-wasm`; on init: `await init()`, `initThreadPool(navigator.hardwareConcurrency)` only if `typeof SharedArrayBuffer !== "undefined"` (single message-type protocol: `{kind:"calc", id, body}` → `{id, ok, result|error}`; `{kind:"options"}` → options; `{kind:"ping"}` → ready signal including `{ threads: boolean }`).
  - `coi.ts`: `maybeRegisterCoi()` — registers `coi-serviceworker.js` with `credentialless` config when `__DEMO__ && !crossOriginIsolated` (the vendored script self-handles the reload). Exports `isolationFailed()` heuristic (SW registered but still not isolated after reload) consumed by Task 9's banner.
- Seed routes: `seed.ts` gains `seedRoutes(calc: typeof submitCalc, getJob)` — after base seed, for geofence ids 1 and 2 run a real calc (`mode: "route"`, category spawnpoint, radius 70, minPoints 3) and store results as route rows (name `"<Fence> Route"`, `geofence_id`, mode `quest`, geometry = MultiPoint of the FC's coordinates, `points` count). Runs post-wasm-ready; skips silently if wasm unavailable (still a usable demo, just no seed routes — log a console.warn).

- [ ] **Step 1:** Failing tests:

```ts
// realtime.test.ts
it("delivers published events to topic subscribers", async () => {
  const got: unknown[] = [];
  const un = demoBus.subscribe("jobs/1", (e) => got.push(e));
  await demoBus.publish("jobs/1", { type: "status", payload: { status: "running" } });
  await new Promise((r) => queueMicrotask(() => r(null)));
  expect(got).toHaveLength(1);
  un();
  await demoBus.publish("jobs/1", { type: "status", payload: { status: "succeeded" } });
  await new Promise((r) => queueMicrotask(() => r(null)));
  expect(got).toHaveLength(1);
});
it("reports connected immediately", () => {
  let status = "";
  demoBus.onStatusChange?.((s) => { status = String(s); });
  expect(status).toBe("connected");
});
```

```ts
// facade.test.ts — worker mocked via injection: facade takes a `postCalc` fn
// (default = real worker); tests inject a fake resolving a canned result.
it("lifecycle: submit → running event → terminal event + getJob result", async () => {
  const events: string[] = [];
  const fake = async () => ({ ok: true as const, result: { data: { type: "FeatureCollection", features: [] }, stats: { total_clusters: 1 } } });
  const { submitCalc, getJob } = createCalcFacade({ postCalc: fake, resolvePoints: async () => [[40, -74]] });
  demoBus.subscribe("jobs/demo-1", (e) => events.push((e.payload as { status: string }).status));
  const id = await submitCalc({ mode: "route", area: EMPTY_FC, clustering: { radius: 70 } });
  expect(id).toBe("demo-1");
  await vi.waitFor(async () => {
    const rec = await getJob(id);
    expect(rec.status).toBe("succeeded");
  });
  expect(events).toContain("running");
  expect(events).toContain("succeeded");
  expect((await getJob(id)).result?.stats).toBeTruthy();
});
it("worker failure → failed record with error", async () => {
  const fake = async () => ({ ok: false as const, error: "boom" });
  const { submitCalc, getJob } = createCalcFacade({ postCalc: fake, resolvePoints: async () => [] });
  const id = await submitCalc({ mode: "route", area: EMPTY_FC });
  await vi.waitFor(async () => expect((await getJob(id)).status).toBe("failed"));
  expect((await getJob(id)).error).toBe("boom");
});
```

- [ ] **Step 2:** `bun run test` → FAIL. Implement `realtime.ts`, `facade.ts` (factory `createCalcFacade(deps)` + default instance wired to the real worker + marker store), `worker.ts`, `coi.ts`. Vendor `coi-serviceworker.js` (grab the canonical single-file script; set `coepCredentialless: true`); keep it in `public/` so it deploys at root scope; registration ONLY under `__DEMO__`.
- [ ] **Step 3:** Point-resolution semantics in `resolvePoints(body)`: mirror the server — cluster/route: markers of `body.category` within `body.area`, filtered by `body.dataFilter?.lastSeen/tth`; bootstrap: no points; reroute/routeStats: use `body.dataPoints`/`body.clusters` as sent. Inject as `dataPoints` (camelCase — `DataPointsArg::Array`).
- [ ] **Step 4:** Wire `index.demo.ts` calc exports + `createRealtimeTransport`; hook `seedRoutes` into `ensureSeeded` completion (idempotent: only when routes store is empty).
- [ ] **Step 5:** `bun run test` + `bun run typecheck` → PASS. Manual spot: `bun run wasm:build` then `bun run dev:demo`, open via Claude Preview (`.claude/launch.json` entry `web-demo`, port 5273 — add it), draw nothing, just verify boot + geofence list + a calc on a seed geofence from the `/map` playground works and the calc dock shows stats. (Preview note: sonner toasts don't auto-dismiss offscreen — known trap, ignore.)
- [ ] **Step 6:** Commit: `feat(web): demo calc — wasm worker, job facade, realtime bus, COI, seed routes`

---

### Task 9: Demo UI chrome — badge, reset, seeding splash, isolation banner

**Files:**
- Create: `apps/web/src/components/demo/demo-badge.tsx`, `apps/web/src/components/demo/demo-boot.tsx` (splash gate)
- Modify: `apps/web/src/main.tsx` (or `App.tsx` — wherever boot composes; gate render on demo boot), `apps/web/src/components/admin/layout/app-bar.tsx` (mount badge in toolbar)

**Interfaces:**
- Consumes: `__DEMO__`, `ensureSeeded`, `resetDemo` from `@api`, `coi.ts` `maybeRegisterCoi`/`isolationFailed`, db `persistent` flag.
- Produces: `<DemoBoot>{children}</DemoBoot>` — when `__DEMO__`: registers COI, awaits `ensureSeeded()` behind a minimal centered splash ("Seeding demo world…"), then renders children; renders a dismissible error banner when `isolationFailed()` (calc disabled, rest of demo usable) and a "edits won't persist" toast once when the db fell back to memory. When not demo: renders children directly (zero overhead, tree-shaken).
- `DemoBadge`: small `Badge` labeled `DEMO` + dropdown/button `Reset demo` → `await resetDemo(); location.reload()`. Mounted in the AppBar toolbar only when `__DEMO__`.

- [ ] **Step 1:** Implement both components (follow existing shadcn/Tailwind idioms in `app-bar.tsx`; Tailwind: scale tokens only). Boot gate composes in `main.tsx` around `<App/>`.
- [ ] **Step 2:** Unit test the gate logic where cheap (e.g. `DemoBoot` renders children immediately when `__DEMO__` is false — set via `vi.stubGlobal`? `__DEMO__` is a compile-time define; in vitest add `define` in the unit project config so it's `false` by default and a tiny demo-flagged test file overrides via direct component prop `forceDemo` — keep the component testable: `<DemoBoot demo={__DEMO__}>`). Assert splash → children transition with a mocked slow `ensureSeeded`.
- [ ] **Step 3:** `bun run test` + `bun run typecheck` green; `bun run build` (live) green — verify badge/splash code absent from live bundle (`grep -ri "Seeding demo world" apps/web/dist` → empty).
- [ ] **Step 4:** Commit: `feat(web): demo chrome — badge, reset, seeding splash, isolation banner`

---

### Task 10: Demo browser smoke (real wasm, third vitest project)

**Files:**
- Modify: `apps/web/vitest.config.ts` (add `demo-browser` project: Playwright browser, include `*.demo.browser.test.*`, alias `@api` → `index.demo.ts`, define `__DEMO__: true`, server headers COOP/COEP credentialless)
- Create: `apps/web/src/demo.demo.browser.test.tsx`
- Modify: `apps/web/package.json` (`"test:demo": "vitest run --project demo-browser"`)

**Interfaces:**
- Consumes: everything. Requires `crates/koji-wasm/pkg` built (`bun run wasm:build` first). Test file guards: if pkg missing, `describe.skip` with a loud console message naming the build command.

- [ ] **Step 1:** Config: copy the existing `browser` project block, adjust name/include/alias/define/headers. Ensure the browser instance context has cross-origin isolation (vitest browser serves through vite — the `server.headers` from the project config apply; verify `crossOriginIsolated === true` inside the first test and only assert-warn, not fail, since threads gracefully degrade).
- [ ] **Step 2:** Write the smoke:

```tsx
// demo.demo.browser.test.tsx
import { describe, expect, it } from "vitest";
import { ensureSeeded } from "@/api/demo/seeds/seed";
import { getAlgorithms, getJob, submitCalc, baseDataProvider } from "@api";

describe("demo mode end-to-end (real wasm)", () => {
  it("seeds, lists geofences, and runs a real wasm cluster calc", async () => {
    await ensureSeeded();
    const fences = await baseDataProvider.getList("geofence", {
      pagination: { page: 1, perPage: 5 }, sort: { field: "id", order: "ASC" }, filter: {},
    });
    expect(fences.total).toBe(32);
    const fence = await baseDataProvider.getOne("geofence", { id: 1 });
    const body = {
      mode: "route", category: "spawnpoint",
      area: { type: "FeatureCollection", features: [{ type: "Feature", properties: {}, geometry: fence.data.geometry }] },
      clustering: { calculationMode: "radius", radius: 70, minPoints: 3 },
    };
    const id = await submitCalc(body);
    let rec = await getJob(id);
    for (let i = 0; i < 200 && !["succeeded", "failed"].includes(rec.status); i++) {
      await new Promise((r) => setTimeout(r, 100));
      rec = await getJob(id);
    }
    expect(rec.status).toBe("succeeded");
    const fc = rec.result?.data as GeoJSON.FeatureCollection;
    expect(fc.type).toBe("FeatureCollection");
    const stats = rec.result?.stats as { total_clusters: number; total_points: number };
    expect(stats.total_points).toBeGreaterThan(100);
    expect(stats.total_clusters).toBeGreaterThan(0);
  });
  it("algorithm options come from wasm", async () => {
    const o = await getAlgorithms();
    expect(o.clustering).toContain("balanced");
    expect(o.routing).toContain("tsp");
    expect(o.bootstrap).toContain("radius");
  });
});
```

- [ ] **Step 3:** `bun run wasm:build` (if pkg stale) then `bun run test:demo` (background, cold ~2min) → PASS. Also run the untouched projects: `bun run test` + `bun run test:browser` → still green.
- [ ] **Step 4:** Commit: `test(web): demo-mode browser smoke over real wasm calc`

---

### Task 11: Deploy workflow + README

**Files:**
- Create: `.github/workflows/deploy-demo.yml`
- Modify: `README.md` (a short "Live demo" section: URL, what runs in-browser, link to spec)

**Interfaces:**
- Produces the Pages deployment. One-time MANUAL repo setting (tell the user at handoff): Settings → Pages → Source = GitHub Actions.

- [ ] **Step 1:** Workflow:

```yaml
name: deploy-demo
on:
  push:
    branches: [main]
  workflow_dispatch:

permissions:
  contents: read
  pages: write
  id-token: write

concurrency:
  group: pages
  cancel-in-progress: true

jobs:
  build-deploy:
    runs-on: ubuntu-latest
    environment:
      name: github-pages
      url: ${{ steps.deployment.outputs.page_url }}
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@nightly
        with:
          targets: wasm32-unknown-unknown
          components: rust-src
      - uses: Swatinem/rust-cache@v2
      - name: Install wasm-pack
        run: curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
      - name: Build wasm
        run: bash crates/koji-wasm/build-wasm.sh
      - uses: oven-sh/setup-bun@v2
      - name: Install web deps
        run: bun install --frozen-lockfile
        working-directory: apps/web
      - name: Build demo SPA
        run: bun run build:demo
        working-directory: apps/web
      - uses: actions/upload-pages-artifact@v3
        with:
          path: apps/web/dist
      - id: deployment
        uses: actions/deploy-pages@v4
```

(Verify `build-wasm.sh` works with a default-nightly toolchain from `dtolnay/rust-toolchain@nightly` — the script calls `rustup run nightly`, which resolves fine. Verify vite outputs to `apps/web/dist`; adjust `path` if the project's outDir differs.)

- [ ] **Step 2:** Validate YAML locally (`actionlint` if available, else careful read). Do NOT push — pushing is outward-facing; the main thread confirms with the user at the end.
- [ ] **Step 3:** README section (3-5 lines): demo URL `https://<owner>.github.io/Koji/`, "clustering/routing/bootstrap run entirely in your browser via WebAssembly", synthetic-data disclaimer, spec link.
- [ ] **Step 4:** Commit: `ci: GitHub Pages demo deployment workflow + README demo section`

---

### Task 12: Final verification sweep + as-built notes

**Files:**
- Modify: `docs/superpowers/specs/2026-07-16-demo-mode-wasm-design.md` (as-built deltas, if any)

- [ ] **Step 1:** Full gates in parallel (background): `cargo test --workspace`, `cargo clippy --workspace`, `cargo fmt --all --check`, `bun run test`, `bun run typecheck`, `bun run test:browser`, `bun run test:demo`, `bun run build`, `bun run build:demo`.
- [ ] **Step 2:** Live-mode regression spot-check via Claude Preview (`web` launch config + local server) — admin boots, one CRUD read works (needs the dev backend running; if no backend available locally, the green browser suite stands in).
- [ ] **Step 3:** Demo verification via Claude Preview (`web-demo`): boot → splash → badge visible → geofence list (32) → `/map` playground calc on a seed fence → stats grid renders → import wizard dry-run → reset demo works. Screenshot proof.
- [ ] **Step 4:** Provenance sweep: `grep -rni dragonite apps/web/src/api docs/superpowers/plans/2026-07-16-demo-mode-wasm.md .github/workflows/deploy-demo.yml` → only pre-existing legitimate Koji uses (e.g. `dragonite_area_id` DB field) — nothing referencing the reference project as a source.
- [ ] **Step 5:** Record as-built deltas in the spec (anything the implementation changed), commit: `docs(spec): demo-mode as-built notes`.

---

## Self-Review (done at plan-writing time)

- **Spec coverage:** §3 boundary → Tasks 4-5; §4.1 validation → recon (done) + Task 2-3 builds; §4.2 exports + shared types → Tasks 1-3; §4.3 threading/COI/COEP → Tasks 5, 8, 9; §4.4 facade → Task 8; §5 demo backend/seeds/markers → Tasks 6-7; §5.3 wasm seed routes → Task 8; §6 build/deploy → Tasks 5, 11; §7 errors → Tasks 8-9 (banner, failed records, idb fallback); §8 testing → per-task TDD + Task 10 + Task 12; §9 order preserved.
- **Placeholders:** none — every step names exact files/commands; two explicitly-flagged read-then-mirror points (`publishRecord` 422 parse, `fetchGeofencesByBbox` signature) direct the implementer to the authoritative current code rather than risk drift from stale copies.
- **Type consistency:** `ApiSurface` names used across Tasks 4-10 (`submitCalc/getJob/getAlgorithms/fetchMarkers/fetchS2Cells/fetchFeatureCollection/fetchGeofencesByIds/fetchGeofencesByBbox/postImport/postConvert/publishRecord/testWebhook/resetDemo/baseDataProvider/authProvider/createRealtimeTransport/loadConfig`) — single source in `types.ts`; wasm exports (`calc/algorithm_options/s2_cells/convert_geometry`) consistent across Tasks 3, 7, 8, 10.
