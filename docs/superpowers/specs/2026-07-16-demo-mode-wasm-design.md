# Demo Mode + WASM Portfolio Deployment — Design

**Date:** 2026-07-16
**Status:** Approved (design review complete, section-by-section)
**Branch:** claude/v2

## 1. Overview

Ship a fully client-side **demo mode** of `apps/web` deployable to GitHub Pages as a portfolio piece. The centerpiece: Koji's clustering / routing / bootstrap algorithms compiled to WebAssembly and running **in the browser**, replacing the server calc pipeline. Every network request routes through a build-time-selected adapter: the `live` adapter keeps today's server behavior byte-for-byte; the `demo` adapter serves a synthetic NYC world from IndexedDB + in-memory generators and executes calc jobs via `koji-wasm`.

### Goals

- Full app works in demo: all 7 admin resources (CRUD), auth, map playground, geofence/route workbenches, import wizard.
- Algorithms crate validated wasm-ready and exposed at **calc parity** (cluster→route, bootstrap, s2 cells) — not just clustering.
- Zero-touch publishing: GitHub Actions builds wasm + SPA on every `main` push, deploys to `https://<user>.github.io/Koji/`.
- Live (server) build unchanged in behavior and free of demo code (tree-shaken).

### Non-Goals

- No server-side changes to API semantics (one shared-crate type move only, see §4.2).
- No real scraped game data shipped — all markers synthetic.
- No SSR/prerender; SPA + hash router as today.
- No demo write-back of calc results to golbat (matches current v2 calc: pure compute).

## 2. Locked Decisions

| Decision | Choice |
|---|---|
| Demo scope | Full app (all resources CRUD + map + workbenches) |
| Seed data | Procedural synthetic markers; NYC area geojson (user-supplied) as seed geofences |
| Threading on Pages | COI service worker shim → keep rayon multithread |
| Persistence | IndexedDB, version-keyed seed, "Reset demo" button |
| Deploy | GitHub project pages, CI on every `main` push |
| Adapter selection | Build-time vite alias (`--mode demo`), provider-level boundary |

## 3. Architecture — `@api` boundary

### 3.1 Layout

```
apps/web/src/api/
  types.ts        — shared surface interface (typed contract both modes implement)
  index.live.ts   — re-exports current impls (existing fetch code relocated; zero behavior change)
  index.demo.ts   — demo impls
  live/           — existing http-calling modules moved here
  demo/
    db.ts         — IndexedDB `koji-demo`, version-keyed seed
    seeds/        — nyc-areas.geo.json + deterministic PRNG marker generator + seeding orchestration
    endpoints/    — per-surface fakes (crud, auth, config, markers, s2, geo-features, import, publish, webhook-test, plugins)
    calc/         — wasm worker host + JobRecord facade
    realtime.ts   — no-op realtime transport
```

### 3.2 Selection mechanism

- `vite build --mode demo` → resolve alias `@api` → `src/api/index.demo.ts`; default/live builds alias to `index.live.ts`.
- Build-time selection tree-shakes cleanly: live bundle contains no wasm/idb/seed code; demo bundle contains no server fetch paths.
- `define: { __DEMO__ }` drives a small demo badge + "Reset demo" button in the UI; `base: '/Koji/'` set only in demo mode.

### 3.3 Call-site refactor (mechanical)

All ~10 network surfaces re-import from `@api`; function signatures unchanged, so `useCalc`, ra-core wiring, and all components are untouched:

| Surface | Today | Demo impl |
|---|---|---|
| `dataProvider` (CRUD ×7 resources) | `/internal/{resource}` | idb stores + JS sort/filter/pagination mirroring server param names |
| `authProvider` | `/internal/auth/*` | auto-authenticated as `demo`; login screen never shown |
| start-center config | `GET /internal/config` | static NYC center |
| `calcClient` (`submitCalc`/`getJob`/`getAlgorithms`) | `/api/v2/jobs`, `/api/v2/algorithms` | wasm worker + job facade (§4.4) |
| markers | `POST /api/v2/golbat-data/{category}` | in-memory synthetic store, bbox/area filtered |
| s2 cells | `POST /api/v2/s2/{level}` | wasm `s2_cells(bbox, level)` |
| geo-features (featurecollection reads, ids/bbox) | `GET /api/v2/{resource}?format=featurecollection` | derived from idb rows |
| import + geometry convert | `POST /internal/import`, `/internal/geometry/convert` | idb transactional write; convert via wasm export (koji-core logic) |
| publish | `POST /internal/{res}/{id}/publish` | set flag + success toast (no downstream) |
| webhook test | `POST /internal/webhooks/{id}/test` | canned success payload |
| plugins list | `GET /internal/plugins` | empty list |
| realtime | WS `/internal/realtime` | no-op transport (reports connected, emits nothing) |

The `index.html`-hash update-check poll is left as-is (works on Pages).

## 4. WASM Layer

### 4.1 Current state (validated by audit)

- `crates/koji-wasm` exists and builds (`build-wasm.sh`: nightly + wasm-pack + `-Z build-std`, wasm-opt disabled to preserve atomics).
- Algorithms crate is wasm-clean: `native` feature gates sysinfo/plugins/std-time; `web_time::Instant` throughout; rayon via `wasm-bindgen-rayon`; 0 unguarded wasm-hostile patterns; tsp-geo/tsp-ils transitive deps compatible.
- Only `cluster()` exported today; routing/bootstrap intentionally excluded.

### 4.2 Exports grow to calc parity

New/extended `koji-wasm` exports:

- `calc(req)` — dispatches the server's three compute paths: cluster→route chain (`run_cluster_route` semantics), bootstrap, and route-of-existing-clusters (reroute). Result `{ data: FeatureCollection, stats }` byte-shape-identical to the server job result.
- `s2_cells(bbox, level)` — s2 crate already in the dep tree; keeps the s2 overlay live.
- `convert_geometry(input, target)` — geometry conversion from koji-core (wasm-safe); keeps import wizard step-2 functional.
- Existing: `version()`, `init_thread_pool`, panic hook.

**Request-type parity (drift-killer, preferred):** extract the calc request types + `resolve()` logic (`CalcRequest`, `ClusterReq`, bootstrap/routing arg-groups) from `koji-service::requests` into a shared crate (target: `algorithms` or `koji-core` — they resolve into algorithms configs and are expected serde-only). Server and wasm then share *identical* request semantics; the server test suite doubles as the drift gate. **Fallback** if extraction hits DB-type tangles: mirrored DTOs in koji-wasm pinned by tests (explicitly second choice).

### 4.3 Threading on GitHub Pages

- `wasm-bindgen-rayon` needs `SharedArrayBuffer` → COOP/COEP headers; Pages cannot set headers.
- **COI service worker** (vendored shim, registered only in the demo `index.html`, copied verbatim to dist root for top-level scope) injects the headers; first visit triggers one instant reload.
- Dev/preview/tests don't need the SW: vite `server.headers` sets real COOP/COEP locally.
- Atomics builds cannot instantiate without SAB → failure path (SW blocked, some private windows) shows a clear error banner, never a silent hang.

### 4.4 Job facade

- Demo `submitCalc` → enqueue in a dedicated module worker, return fake job id.
- `getJob` reads a worker-side job map; coarse phase progress (resolving points → clustering → routing) mapped to `JobRecord.progress/phase`.
- Point resolution mirrors the server: server pulls golbat points pre-enqueue; demo filters the synthetic marker store by point-in-polygon against the request area.
- `getAlgorithms` → static const matching server enums, pinned by a unit test against the wasm-exposed mode list.

## 5. Demo Backend

### 5.1 IndexedDB

- DB `koji-demo`, **version-keyed**: version bump on deploy → automatic wipe + reseed (stale data never poisons a new deploy).
- Stores: geofences, routes, projects, properties, webhooks, tile-servers.
- `getList` implements sort/filter/pagination in JS mirroring server params.
- idb unavailable (private mode/quota) → in-memory fallback + "edits won't persist" toast.

### 5.2 Synthetic markers

- **Not stored**: regenerated at boot in-memory. Deterministic seeded PRNG; density-clustered inside seed geofences + street-noise scatter; all 4 categories (spawnpoint, gym, pokestop, station). Tens of thousands of points — in-memory bbox filtering is fine.
- No real scraped data ships.

### 5.3 Seed world

- NYC area geojson (checked in at `apps/web/src/api/demo/seeds/nyc-areas.geo.json`) → demo geofences with modes assigned; 2–3 projects grouping them.
- **Seed routes are computed by real wasm calc at first seed** (brief "seeding demo…" splash) — genuine algorithm output, not canned geometry.
- "Reset demo" wipes idb, reseeds, reloads.

## 6. Build & Deploy

### 6.1 Local

- `bun run wasm:build` — wraps `crates/koji-wasm/build-wasm.sh` (nightly + wasm-pack).
- `bun run dev:demo` — `vite --mode demo`; serverless iteration. Web imports the wasm pkg via vite alias to `crates/koji-wasm/pkg`.

### 6.2 CI (`.github/workflows/deploy-demo.yml`)

- Triggers: push to `main` + `workflow_dispatch`.
- Steps: checkout → nightly toolchain + wasm32 target + wasm-pack (cargo cached) → `build-wasm.sh` → `bun install` → `vite build --mode demo` (`base: '/Koji/'`) → `upload-pages-artifact` → `deploy-pages`.
- No gh-pages branch; one-time repo setting: Pages source = GitHub Actions.
- wasm + worker assets hashed through the vite pipeline (cache-safe per deploy); COI SW copied to dist root unhashed.
- Hash router → no 404.html trick needed.

## 7. Error Handling

| Failure | Behavior |
|---|---|
| SAB unavailable after COI SW attempt | Visible error banner with reload hint; never a hang |
| Wasm panic | `console_error_panic_hook` + worker catch → `JobRecord{status:"failed", error}` → existing UI error path |
| Worker/module load failure (stale cache) | calcClient error state, surfaced in the calc dock |
| idb unavailable / quota | In-memory fallback store + non-persistence toast; demo stays usable |
| Empty area / zero points | Same as server: zero-point stats (algorithms already handle) |
| Seed version skew | Silent wipe + reseed |

## 8. Testing

- **Rust:** koji-wasm unit tests extended per new export (calc dispatch, bootstrap, s2, convert) — native `cargo test`, existing pattern. Shared request-types extraction keeps the server suite green as the drift gate.
- **Web unit (vitest):** demo CRUD sort/filter/pagination; PRNG determinism (same seed → identical points); job facade lifecycle (submit → poll → result); `getAlgorithms` pin test.
- **Browser (vitest browser):** existing suite runs live-mode unchanged. New demo smoke: boot `--mode demo`, seed, run a real wasm cluster calc, assert rendered clusters — end-to-end of the showcase path. Local COOP/COEP via vite headers (no SW in tests).
- Full suites at phase boundaries only.

## 9. Implementation Order (sketch for planning)

1. Shared request-types extraction (or fallback DTOs) + koji-wasm export growth + tests.
2. `@api` boundary refactor (live mode only, behavior-neutral) + green suite.
3. Demo backend: db/seeds/endpoints + unit tests.
4. Calc worker + job facade + COI wiring + browser smoke.
5. Deploy workflow + Pages setup + live verification.
