# v2 API — Phase 6: Frontend migration (v1/internal → v2) — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:executing-plans. This is a TypeScript/React migration — verified by `tsc --noEmit` + the frontend build (type-first), NOT cargo TDD. Steps use `- [ ]`.

**Goal:** Move the in-repo frontend (`client/`, React + react-admin + Vite) off the v1 + `/internal` API onto v2, so v1 can be torn down (P7). Migrate the centralized `fetches.ts`, the react-admin `dataProvider.ts`, the calc flow (sync → async job+poll), the envelope handling, auth/config, and the ~35 call sites the scout mapped.

**⚠️ Verification ceiling:** this environment has **no running backend + no DB**, so runtime behavior is NOT verifiable here. Verify **type-first**: update the response types to the v2 shapes, then make `tsc --noEmit` + the production build pass — the compiler catches shape/import errors at every call site. **Runtime correctness (URL strings, the job-poll loop, react-admin's data contract) requires a real-deploy smoke before merge — flag everything uncertain.**

**Tech Stack:** TypeScript, React, react-admin, Vite, zustand (`useStatic`).

## Delegate-mode decisions (recorded)

- **Type-first migration.** Update `KojiResponse`/job/envelope types to v2, then fix the cascade of `tsc` errors. This is the safety net.
- **Envelope adapter centralized in `fetchWrapper`** — unwrap `{status:"ok",data,meta?}` → `data` (+ surface `meta`); on `{status:"error",error}` / `!res.ok` → notify with `error.message`. Raw `?format=` exports (e.g. `sql` text) handled where those specific calls are made.
- **`save-scanner` / `push/{id}` → `POST /api/v2/{resource}/{id}/publish`** as the BEST-GUESS mapping, with a `// TODO(v2-verify): publish vs scanner-sync semantics` comment at each site. NOT resolved blind — flagged for the user's smoke.
- **`/internal/routes/from_scanner` (scanner-sourced routes)** has no v2 backend equivalent → leave the call commented with a `// TODO(v2-gap): no v2 endpoint` + a stub returning empty, so the build passes; flagged.
- **react-admin `dataProvider`** adapts to v2: `getList` → `{data, total}` (total from `meta`), `page/perPage` → `?page=&per_page=`, sort/filter → `?sortBy=&order=&q=`.

---

## Task 1: Types + envelope adapter (`fetchWrapper`)

**Files:** `client/src/services/fetches.ts`, `client/src/types.ts` (or wherever `KojiResponse`/API types live — grep for them).

- [ ] **Step 1** — Read `fetches.ts` fully + the API types. Add v2 envelope types: `interface ApiOk<T> { status: 'ok'; data: T; meta?: ApiMeta }`, `interface ApiErr { status: 'error'; error: { code: string; message: string; field?: string } }`, `interface ApiMeta { total: number; page: number; per_page: number; total_pages: number; has_next: boolean; has_prev: boolean }`, and a `JobRecord` type (`{ id; status: 'queued'|'running'|'succeeded'|'failed'|'canceled'; progress: number; phase?: string; result?: { data: FeatureCollection | null; stats: KojiStats }; error?: string }`).
- [ ] **Step 2** — Rewrite `fetchWrapper<T>` to unwrap the v2 envelope: parse JSON; if `json.status === 'error'` or `!res.ok` → push the error notification (`json.error?.message`) and return `null`; else return `json.data as T`. Preserve the existing auth-header/credentials behavior.
- [ ] **Step 3** — `tsc --noEmit` (expect errors at call sites — fixed in later tasks). Commit.

```bash
git add client/src && git commit -m "feat(client): v2 envelope types + fetchWrapper unwrap"
```

---

## Task 2: Calc flow — sync → async job+poll

**Files:** `client/src/services/fetches.ts` (`clusteringRouting`), any caller (`useImportExport.ts`, `Point.tsx`).

- [ ] **Step 1** — Add `pollJob(jobId: string, signal?: AbortSignal): Promise<JobRecord>` — loops `GET /api/v2/jobs/{id}?wait=10` until `status` is terminal (`succeeded`/`failed`/`canceled`) or aborted; on abort, `DELETE /api/v2/jobs/{id}`. Returns the terminal `JobRecord`.
- [ ] **Step 2** — Refactor `clusteringRouting()`: build the v2 `CalcJobRequest` body `{ mode, category, area, clustering, routing, bootstrap?, output, dataFilter, dev }` from the current request params (read how it builds the v1 body + map fields to the nested arg-groups; the v2 arg-groups are camelCase `clustering.radius`/`routing.sortBy`/etc.). For each area: `POST /api/v2/jobs` → `{ job_id }` → `pollJob(job_id)` → read `result.data` (FeatureCollection) + `result.stats`. Keep the `Promise.allSettled` multi-area parallelism (now over jobs). Map the existing AbortController to `pollJob`'s signal.
- [ ] **Step 3** — Map the calc modes: v1 `/calc/{mode}/{category}` + `/calc/bootstrap` + `/calc/reroute` + `/calc/route-stats/{category}` → v2 job `mode` ∈ `cluster|route|bootstrap|reroute|routeStats` with `category` in the body.
- [ ] **Step 4** — `tsc --noEmit` clean for these files. Commit.

```bash
git add client/src && git commit -m "feat(client): calc via v2 async jobs (POST /jobs + pollJob)"
```

---

## Task 3: Geometry / S2 / markers / area

**Files:** `fetches.ts` (`convert`, `getS2Cells`, `s2Coverage`, `getMarkers`), `Polygon.tsx`, `Point.tsx`.

- [ ] **Step 1** — Swap URLs to v2:
  - `convert` → `POST /api/v2/geometry/convert` (pass `?format=` for non-geojson outputs); merge-points → `POST /api/v2/geometry/merge-points`; polygon area → `POST /api/v2/geometry/area`.
  - `getS2Cells`/`s2Coverage` → `POST /api/v2/s2/{level}` | `/s2/circle-coverage` | `/s2/cell-coverage` (these stay synchronous).
  - `getMarkers` → `POST /api/v2/scanner-data/{category}` with `{ area | bbox, lastSeen, tth }`; area-stats → `POST /api/v2/scanner-data/{category}/stats`.
- [ ] **Step 2** — `tsc --noEmit` clean. Commit.

```bash
git add client/src && git commit -m "feat(client): geometry/s2/scanner-data on v2 endpoints"
```

---

## Task 4: react-admin `dataProvider` + resource CRUD

**Files:** `client/src/.../dataProvider.ts` + the admin filters/forms (`GeofenceFilter.tsx`, `RouteFilter.tsx`, `GeofenceForm.tsx`, `Settings.tsx`, `AssignParentFence.tsx`, `AssignProjectFence.tsx`, `Export.tsx`, etc.).

- [ ] **Step 1** — Rewrite the dataProvider URL map: `/internal/admin/{resource}/*` → `/api/v2/{geofences|routes|projects|properties|tile-servers}`. `getList` → `GET ?page=&per_page=&sortBy=&order=&q=` then `{ data: json.data, total: json.meta.total }`. `getOne` → `data`. `create`/`update` → `data`. `delete` → 204 → synthesize `{ data: { id } }`. `getMany`/bulk → `?per_page=…` or parallel.
- [ ] **Step 2** — Migrate the specific calls: parent geofences → `GET /api/v2/geofences?...` (drop the route-parent lookup → `// TODO(v2-gap): routes have no parent`), properties → `GET /api/v2/properties`, tile servers → `GET /api/v2/tile-servers`, project search → `GET /api/v2/projects?q=`, assign-parent → `PATCH /api/v2/geofences/{id} { parent }`, assign-project → `PATCH /api/v2/geofences/{id} { projects }`, export → `GET /api/v2/{resource}/{id}?format=…`.
- [ ] **Step 3** — `tsc --noEmit` clean. Commit.

```bash
git add client/src && git commit -m "feat(client): react-admin dataProvider + admin calls on v2"
```

---

## Task 5: Auth / config + the flagged maps

**Files:** `App.tsx`, `Login.tsx`, `Settings.tsx`, `SaveToKoji.tsx`, `SaveToScanner.tsx`, `PushToApi.tsx`, `Nominatim.tsx`, `getScannerCache`/`getKojiCache` in `fetches.ts`.

- [ ] **Step 1** — Auth/config: `GET /config/` → `GET /api/v2/config`; `POST /config/login` → `POST /api/v2/auth/login`; logout link → `POST /api/v2/auth/logout`; Nominatim → keep `/config/nominatim` for now (or `/api/v2/...` if a v2 route exists — else leave + flag).
- [ ] **Step 2** — save-koji → `POST /api/v2/geofences` / `POST /api/v2/routes` (loop the drawn features; one create per feature). cache refreshers (`getKojiCache`) → `GET /api/v2/{resource}?per_page=9999`.
- [ ] **Step 3** — The FLAGGED maps: `save-scanner` + `push/{id}` → `POST /api/v2/{resource}/{id}/publish` with a `// TODO(v2-verify)` comment; `getScannerCache` (`/internal/routes/from_scanner`) → stub-empty + `// TODO(v2-gap)`.
- [ ] **Step 4** — `tsc --noEmit` clean. Commit.

```bash
git add client/src && git commit -m "feat(client): auth/config on v2; save/publish best-guess maps (flagged)"
```

---

## Task 6: Build verification

- [ ] **Step 1** — Read `client/package.json` for the scripts. Run the type-check + production build (typically `npm run build` / `tsc -b && vite build`; use the repo's package manager — pnpm/npm/yarn per the lockfile). Install deps first if needed.
- [ ] **Step 2** — Iterate until **`tsc --noEmit` is clean and the build succeeds**. Then:

```bash
rg -n "/api/v1/|/internal/" client/src || echo "CLEAN: no v1//internal calls remain (except flagged TODOs)"
```
Expected: only the flagged `TODO(v2-gap)` stubs remain (list them).

- [ ] **Step 3** — Commit + report the full list of `TODO(v2-verify)` / `TODO(v2-gap)` flags and every endpoint whose runtime behavior is unverified.

```bash
git add client && git commit -m "chore(client): v2 migration build-green; flag runtime-unverified sites"
```

---

## Self-Review

**Spec coverage:** frontend off v1/`/internal` → v2 (§P5 of the design) ✓ · calc async-poll ✓ · envelope adapter ✓ · dataProvider ✓ · auth/config ✓ · gap endpoints wired ✓. Domain-ambiguous maps (save-scanner/push/from_scanner) flagged, not guessed-as-fact.

**Verification honesty:** type-checked + build-verified only; runtime requires a real-deploy smoke (stated up front + per flagged site). This is the agreed ceiling for a no-backend/no-DB environment.

**Consistency:** v2 envelope `{status,data,meta}`, job record shape, `?page=&per_page=`, camelCase calc arg-groups, `?format=` — all match the P0–P5 backend as built.
