# Koji v2 Map — Phase 3: Calc + Jobs (all 5 modes)

Branch `claude/v2`. Extends the map (Phase 1 read-only, Phase 2 editing) with the
compute surface: run Koji's clustering/routing/bootstrap as async jobs, watch live
progress, overlay the result.

## Backend contract (verified in `crates/koji-service/src/public/v2/{jobs,calc}.rs`, `requests/ops.rs`, `groups.rs`)

- `POST /api/v2/jobs` body = flattened `CalcRequest` + `category`:
  - `{ mode, category, ... }`, `mode` ∈ `cluster|route|reroute|bootstrap|routeStats` (camelCase tag).
  - cluster/route: `area` (geojson FC/Feature/Geometry, `[lon,lat]`) **or** `parent` (id) **or** `dataPoints` (`[[lat,lon]]`); `clustering:{radius,minPoints,mode,…}`; `routing:{sortBy}`; `dataFilter:{lastSeen,tth}`; `instance`.
  - bootstrap: `area`; `bootstrap:{radius,calculationMode,…}`; `routing`.
  - reroute: `clusters:[[lat,lon]]` (+ optional `dataPoints`); `routing`; `radius`.
  - routeStats: `clusters:[[lat,lon]]` (+ optional `dataPoints`); `radius`; `minPoints`.
  - → `202 { status:"ok", data:{ job_id } }` (+ `Location` header).
- `GET /api/v2/jobs/{id}` → record `{ id, kind, status, progress(0..1), phase, result:{data:<geojson FC>, stats}|null, error|null }`. `?wait=N` long-polls.
- `DELETE /api/v2/jobs/{id}` → cancel.
- `GET /api/v2/algorithms` → `{ clustering:[…], routing:[…], bootstrap:[…] }` mode-string lists.
- Realtime topic `jobs/{id}` → `{ id, status, progress, phase }` on status+progress. (`jobs` global exists too — used by dashboard.)
- Result `data` = geojson `FeatureCollection` of MultiPoint cluster centers (+ route order); the wire is standard `[lon,lat]`.

Client auth: `apiV2Fetch` (`@/lib/http`) — session-cookie, `/api/v2` base. Realtime: `useSubscribe(topic, cb)` (`@/components/realtime`), `cb(event)`, `event.payload`.

## Coordinate gotcha
- `area` in / `result.data` out are geojson `[lon,lat]`.
- `clusters`/`dataPoints` are koji `SingleVec` `[lat,lon]` — **transpose** a selected route's geojson coords `[lon,lat]`→`[lat,lon]` when feeding reroute/routeStats.

## State (new `useMapCalcStore` — separate domain, S3 per-field in leaves)
`mode, category, radius, minPoints, clusterMode, sortBy, areaSource("viewport"|"selected"), job{id,status,progress,phase}|null, resultFC|null, stats|null, error|null` + actions.
Rationale: keep the calc domain out of `map-ui-store`; panels subscribe per-field (S3).

## Slices (TDD; commit each)
1. **Data core** — `stores/map-calc-store.ts` (+test), `lib/calc-request.ts` (pure: `boundsToAreaFC`, `routeCoordsToClusters` transpose, `buildCalcBody(state, {area,clusters})`, `parseCalcResult(record)`) (+test), `data/calc-client.ts` (`submitCalc`, `getJob`, `getAlgorithms` over `apiV2Fetch`) (+test w/ mocked fetch).
2. **Job hook** — `data/use-calc-job.ts`: on `job.id`, `useSubscribe("jobs/"+id)` → update status/progress/phase; on `succeeded` → `getJob` → `setResult`; on `failed` → `setError` (+browser test w/ fake subscribe).
3. **Panel** — `panels/calc-panel.tsx`: mode select (5), category (cluster/route/bootstrap), radius/minPoints, cluster-mode + sortBy selects (from `getAlgorithms`), area-source toggle; reroute/routeStats use the selected route. Calculate → submit → store.job. Progress bar + phase + stats + Cancel/Clear (+browser test).
4. **Overlay + wire** — `lib/layers.ts` add a `calc-result` layer (centers ScatterplotLayer + route GeoJsonLayer line, distinct color) gated on `resultFC`; `deck-canvas.tsx` subscribe `resultFC` + `useCalcJob()`; `map-route.tsx` mount `<CalcPanel/>`.
5. **Verify** — typecheck + unit + browser + build; live-verify in the user's tab; commit; update memory.

## Assumptions (delegate-mode calls — flag any to change)
- **Overlay-only** result (no auto-save-as-route this pass; a "Save result as route" button is a fast follow).
- reroute/routeStats input = the **selected route**'s coords → `clusters`; empty `dataPoints` (distance routing/stats only) for MVP.
- cluster/route/bootstrap area = **current viewport bbox** by default, or the **selected geofence** if `areaSource="selected"`.
- Progress via the **`jobs/{id}` realtime** topic (falls back to a single `getJob` on mount); no polling loop.
- Cluster-mode / sortBy option lists come from `GET /algorithms`; if it fails, those selects hide and the server defaults apply.
- New `useMapCalcStore` rather than extending `map-ui-store` (separate domain).
