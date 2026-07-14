# Overlap Visualization — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add read-only geofence overlays for overlap visualization — a member-geofence map on project pages, and a ghost-neighbor toggle on geofence pages — backed by two scoped `GET /api/v2/geofences` query params so a 600-fence org never loads all fences.

**Architecture:** Backend gains `?ids=` (DB-level, only requested rows) and `?bbox=` (in-memory intersect filter) on the geofences read. Frontend adds a shared read-only interactive geofences overlay (deck.gl `getTooltip` for hover→name, `window.open` for click→new-tab edit, dimmed "ghost" styling), consumed by a project map (member fences) and a neighbor toggle hook (bbox-scoped, ghost).

**Tech Stack:** Rust (actix, sea-orm, koji-service/koji-db/koji-core), React 19, react-admin/shadmin-core, deck.gl v9 + maplibre, TanStack Query, Vitest browser provider, bun.

**Spec:** `docs/superpowers/specs/2026-07-14-overlap-viz-design.md`

## Global Constraints

- No DB migration (bbox columns/index = separate follow-up chip). `?bbox=` filters in-memory; `?ids=` filters at the DB via `Id.is_in`.
- No reuse of `useGeoFeatures` fetch-all for these surfaces (would load all 600 fences).
- Bounds order everywhere is geojson `[minLng, minLat, maxLng, maxLat]` (matches `geometryBounds` in `components/deck/bounds.ts`). The `?bbox=` param is that, comma-joined.
- Feature name at `properties.name`; feature id at `feature.id ?? feature.properties.id`.
- Click→edit opens a NEW tab at the hash-router edit URL: `window.open(`${location.origin}${location.pathname}#/geofence/${id}`, "_blank", "noopener")`.
- Neighbor toggle default OFF; neighbors dimmed "ghost"; project member fences drawn normal.
- Frontend cmds: `cd apps/web && bun run test <file>` (unit/jsdom), `bun run test:browser <file>` (Chromium), `bun run typecheck`. Browser tests that click over the deck canvas must `import "@/index.css"`.
- Backend cmds: `cargo test -p koji-service <name>`. DB-touching tests gate on `KOJI_DB_URL` (see memory `koji-test-db-setup`: reconstruct `.env.test`, `cargo run -p migration -- up`); pure-function tests need no DB.
- Run heavy suites once at task end, background if >5s.

---

### Task 1: Backend — `?ids=` and `?bbox=` filters on `GET /api/v2/geofences`

**Files:**
- Modify: `crates/koji-service/src/public/v2/geofences.rs` (`ReadQuery` ~:43, `list()` ~:140)
- Modify: `crates/koji-db/src/db/geofence/reads.rs` (add an ids-filtered read alongside `get_all_koji` :139)
- Create/Modify: a pure bbox-intersect helper (in `geofences.rs` or a small module) + its unit test.

**Interfaces:**
- Produces: `GET /api/v2/geofences?ids=1,2,3` → FeatureCollection of only those fences; `?bbox=minLng,minLat,maxLng,maxLat` → only fences whose geometry bbox intersects; neither → all (unchanged). Same envelope/`?format=`/`properties.name`/feature-`id` as today.

- [ ] **Step 1 — Pure bbox filter + failing unit test.** Add a pure helper `fn features_intersecting_bbox(fc: FeatureCollection, bbox: [f64;4]) -> FeatureCollection` that keeps features whose own bbox (compute via `koji_core` bbox primitives — `KojiGeometry::bbox()` or `koji_output::geojson_bbox`; pick what a `geojson::Feature` gives cleanly) overlaps `bbox` (standard AABB overlap: `!(aMaxX < bMinX || aMinX > bMaxX || aMaxY < bMinY || aMinY > bMaxY)`). Write a unit test (no DB): a FC with two features (one inside the box, one far outside) → only the inside one survives; a feature straddling the boundary → survives.
- [ ] **Step 2 — Run, verify it fails.** `cargo test -p koji-service features_intersecting_bbox` → FAIL (undefined).
- [ ] **Step 3 — Implement the helper** per above; parse nothing here (pure geometry).
- [ ] **Step 4 — Run, verify pass.**
- [ ] **Step 5 — Wire the handler + db read.** In `reads.rs` add `pub async fn get_koji_by_ids(db, ids: &[u32]) -> Result<..>` mirroring `get_all_koji` but `Entity::find().filter(geofence::Column::Id.is_in(ids.iter().copied()))` before the same conversion. In `ReadQuery` add `ids: Option<String>` + `bbox: Option<String>`. In `list()`: parse `ids` (split `,`, trim, parse u32, ignore blanks) → `get_koji_by_ids`; else parse `bbox` (4 f64s) → `get_all_koji` then `features_intersecting_bbox`; else current `get_all_koji`. On malformed params return a 400 (match the crate's existing error helper). Add a `// ponytail: in-memory bbox filter; DB bbox columns + index if fence counts grow (follow-up chip)` comment.
- [ ] **Step 6 — Integration test (DB-gated).** If `KOJI_DB_URL` is set / the koji-service test fixture (used by the webhooks tests, e.g. `test_db_app`) is available, add a test: seed 2–3 geofences, assert `?ids=<one id>` returns exactly that feature, `?bbox=` returns only intersecting, no-param returns all. If the local test DB is unavailable, note it in the report and rely on the pure unit test + preview verification — do not block.
- [ ] **Step 7 — Run** `cargo test -p koji-service geofence` (+ the pure test) and `cargo clippy -p koji-service` → clean.
- [ ] **Step 8 — Commit.** `feat(service): add ?ids= and ?bbox= filters to GET /api/v2/geofences`

---

### Task 2: `DeckMap` `getTooltip` prop

**Files:**
- Modify: `apps/web/src/components/deck/deck-map.tsx` (`DeckMapProps` :40-50, `<DeckGL>` :120-150)
- Test: `apps/web/src/components/deck/deck-map.browser.test.tsx` (extend — created in the prior feature)

**Interfaces:**
- Produces: `DeckMapProps.getTooltip?: (info: PickingInfo) => { text: string } | null`, forwarded verbatim to `<DeckGL getTooltip={getTooltip}>`. Omitted → undefined → no tooltip (back-compat).

- [ ] **Step 1 — Failing test.** Render `<DeckMap layers={[oneScatterOrGeoJsonLayer]} getTooltip={() => ({ text: "hello" })} />`. deck.gl renders the tooltip into a DOM node with class `deck-tooltip` only on hover, which is hard to trigger reliably in the harness — instead assert the wiring at the props level: render with and without `getTooltip` and assert no crash + `[data-testid="deck-map"]` present in both; and add a focused assertion that the DeckGL element received the prop by spying/mocking `@deck.gl/react`'s default export to capture props (mirror how `map-index`/other tests mock deck modules), asserting the captured `getTooltip` is the passed function when provided and `undefined` when not.
- [ ] **Step 2 — Run, verify it fails.** `cd apps/web && bun run test:browser deck-map`.
- [ ] **Step 3 — Implement.** Add `getTooltip` to props + destructure; pass `getTooltip={getTooltip}` to `<DeckGL>`. No other change.
- [ ] **Step 4 — Run, verify pass.** Typecheck: `bun run typecheck`.
- [ ] **Step 5 — Commit.** `feat(web): thread getTooltip through DeckMap to DeckGL`

---

### Task 3: `openGeofenceEdit` util + `geofenceOverlayLayer` builder

**Files:**
- Create: `apps/web/src/map/lib/open-geofence.ts`
- Create: `apps/web/src/map/lib/geofence-overlay.ts`
- Test: `apps/web/src/map/lib/geofence-overlay.test.tsx` (unit)

**Interfaces:**
- Consumes: `GeoJsonLayer` (`@deck.gl/layers`), `PickingInfo`/`Layer` (`@deck.gl/core`), the `EDITING_FILL`/`EDITING_LINE`/`GEOFENCE_FILL`/`GEOFENCE_LINE` RGBA consts (`@/map/lib/layers` or `map-colors` — grep for where they live).
- Produces:
  - `openGeofenceEdit(id: number | string): void` — `window.open(`${location.origin}${location.pathname}#/geofence/${id}`, "_blank", "noopener")`.
  - `geofenceOverlayLayer(opts: { features: GeoJSON.Feature[]; ghost: boolean; excludeId?: number | string | null; id?: string }): Layer` — a `GeoJsonLayer` (id defaults `"geofence-overlay"`), `pickable:true`, filled+stroked, ghost→`EDITING_*` (dim gray) / normal→`GEOFENCE_*` (orange), `data` = features minus the one whose `f.id ?? f.properties?.id` equals `excludeId`, `onClick: (info) => { const id = info.object?.id ?? info.object?.properties?.id; if (id != null) openGeofenceEdit(id); }`.
  - `overlayTooltip(info: PickingInfo): { text: string } | null` — `info.object?.properties?.name ? { text: String(info.object.properties.name) } : null`.

- [ ] **Step 1 — Failing tests.** (a) `openGeofenceEdit(7)`: stub `window.open` (`vi.spyOn`), stub `window.location` origin/pathname, assert it's called once with `".../#/geofence/7"`, `"_blank"`, `"noopener"`. (b) `geofenceOverlayLayer({features:[A,B], ghost:true, excludeId: A.id})`: assert the layer's `data` excludes A, includes B; assert `getFillColor`/`getLineColor` resolve to the dim `EDITING_*` values; `ghost:false` → orange `GEOFENCE_*`. (c) `overlayTooltip`: object with `properties.name:"X"` → `{text:"X"}`; object without name → null.
- [ ] **Step 2 — Run, verify it fails.** `cd apps/web && bun run test open-geofence geofence-overlay` (or the single test file) → FAIL.
- [ ] **Step 3 — Implement** both files per the interfaces.
- [ ] **Step 4 — Run, verify pass.** Typecheck.
- [ ] **Step 5 — Commit.** `feat(web): geofence overlay layer builder + new-tab edit opener`

---

### Task 4: Data hooks `useGeofencesByIds` + `useGeofencesByBbox`

**Files:**
- Modify: `apps/web/src/map/data/use-geo-features.ts` (add two hooks + their fetchers alongside `fetchFeatureCollection`)
- Test: `apps/web/src/map/data/use-geo-features.test.tsx` (unit; extend if present)

**Interfaces:**
- Consumes: `apiV2Fetch` (`@/lib/http`) — returns `{ status, json }`; unwrap `json.data ?? json` (mirror `fetchFeatureCollection` :6-13). `useQuery` (`@tanstack/react-query`). `Bounds` (`@/map/stores/types`).
- Produces:
  - `useGeofencesByIds(ids: (number|string)[], enabled?: boolean)` → react-query result of `FeatureCollection`. Query key `["geo","geofences","ids", [...ids].sort()]`. `enabled: (enabled ?? true) && ids.length > 0`. Fetcher: `apiV2Fetch(`/geofences?format=featurecollection&ids=${ids.join(",")}`)`.
  - `useGeofencesByBbox(bbox: Bounds | null, enabled: boolean)` → react-query result of `FeatureCollection`. Query key `["geo","geofences","bbox", bbox]`. `enabled: enabled && !!bbox`. Fetcher: `apiV2Fetch(`/geofences?format=featurecollection&bbox=${bbox!.join(",")}`)`.
  - Both unwrap via the same `j.data ?? (j.type==="FeatureCollection" ? j : EMPTY)` logic; a non-2xx status returns `EMPTY`.

- [ ] **Step 1 — Failing test.** Mock `@/lib/http`'s `apiV2Fetch` (`vi.mock`). Using the project's hand-rolled `renderHook` (see `components/deck/use-calc.test.tsx`) wrapped in a `QueryClientProvider`, render `useGeofencesByIds([2,1])` → assert `apiV2Fetch` called with a URL containing `ids=1,2` (or `2,1` — assert the ids you pass are present) and returns the mocked FC; render with `[]` → assert NOT called (disabled). Render `useGeofencesByBbox([0,0,1,1], true)` → URL contains `bbox=0,0,1,1`; `useGeofencesByBbox(null, true)` → not called.
- [ ] **Step 2 — Run, verify it fails.** `cd apps/web && bun run test use-geo-features` → FAIL.
- [ ] **Step 3 — Implement** the two hooks + fetchers.
- [ ] **Step 4 — Run, verify pass.** Typecheck.
- [ ] **Step 5 — Commit.** `feat(web): useGeofencesByIds + useGeofencesByBbox data hooks`

---

### Task 5: `useNeighborOverlay` hook

**Files:**
- Create: `apps/web/src/components/deck/use-neighbor-overlay.ts`
- Test: `apps/web/src/components/deck/use-neighbor-overlay.test.tsx` (unit)

**Interfaces:**
- Consumes: `useGeofencesByBbox` (Task 4), `geofenceOverlayLayer` + `overlayTooltip` (Task 3), `geometryBounds` (`./bounds`), `Bounds`/`Layer`.
- Produces: `useNeighborOverlay(geometry: GeoJSON.Geometry | null | undefined, currentId?: number | string | null)` → `{ on: boolean; setOn: (v:boolean)=>void; layers: Layer[]; getTooltip: (info)=>({text:string}|null); label: "Neighbors" }`.
  - Default `on=false`. Compute `bbox` = `geometry ? padBbox(geometryBounds(geometry), 0.2) : null` where `padBbox([minX,minY,maxX,maxY], f)` expands each axis by `f * span` on both sides. `useGeofencesByBbox(bbox, on && !!geometry)`. `layers` = `on && data ? [geofenceOverlayLayer({ features: data.features, ghost:true, excludeId: currentId })] : []`. `getTooltip = overlayTooltip`.

- [ ] **Step 1 — Failing test.** Mock `useGeofencesByBbox` (return a 2-feature FC when enabled). Hand-rolled `renderHook`. Assert: default `on===false`, `layers.length===0`; after `setOn(true)` with a non-null `geometry`, `layers.length===1` and the excluded `currentId` feature is absent from the layer data; with `geometry` null, `setOn(true)` still yields `layers.length===0` (no bbox → no fetch). Assert `padBbox` widens a `[0,0,10,10]` box (e.g. to `[-2,-2,12,12]` at f=0.2).
- [ ] **Step 2 — Run, verify it fails.** `cd apps/web && bun run test use-neighbor-overlay` → FAIL.
- [ ] **Step 3 — Implement** per interfaces; export `padBbox` for the test (or test it via the hook).
- [ ] **Step 4 — Run, verify pass.** Typecheck.
- [ ] **Step 5 — Commit.** `feat(web): useNeighborOverlay hook (bbox-scoped ghost neighbors, default off)`

---

### Task 6: `ProjectGeofencesMap` component

**Files:**
- Create: `apps/web/src/components/deck/project-geofences-map.tsx`
- Test: `apps/web/src/components/deck/project-geofences-map.browser.test.tsx`

**Interfaces:**
- Consumes: `useGeofencesByIds` (Task 4), `geofenceOverlayLayer` + `overlayTooltip` (Task 3), `DeckMap` + its new `getTooltip`/`expandable` (Task 2 + prior feature), `geometryBounds`/union-bounds util (`./bounds`).
- Produces: `ProjectGeofencesMap({ ids }: { ids: (number|string)[] })` — `useGeofencesByIds(ids, ids.length>0)`; layers = `[geofenceOverlayLayer({ features: data?.features ?? [], ghost:false })]`; render a `DeckMap` (height 640, `expandable`, `getTooltip={overlayTooltip}`, `fitBounds` = the bbox spanning all member features — compute from the FC via `geometryBounds` on the FeatureCollection). Empty `ids` → render a small bordered placeholder "No geofences" (reuse DeckGeoJsonField's empty-state classes).

- [ ] **Step 1 — Failing test.** `import "@/index.css"`. Mock `useGeofencesByIds` to return a 2-feature FC. Render `<ProjectGeofencesMap ids={[1,2]} />` inside `AdminContext`/`QueryClientProvider` → assert `[data-testid="deck-map"]` present. Render `ids={[]}` → assert the "No geofences" placeholder (no deck-map). Hover behavior/tooltip is not asserted (deck canvas) — covered by the `overlayTooltip` unit test in Task 3.
- [ ] **Step 2 — Run, verify it fails.** `cd apps/web && bun run test:browser project-geofences-map` → FAIL.
- [ ] **Step 3 — Implement** per interfaces.
- [ ] **Step 4 — Run, verify pass.** Typecheck.
- [ ] **Step 5 — Commit.** `feat(web): ProjectGeofencesMap (member fences, read-only, hover+click-to-edit)`

---

### Task 7: Wire the project map into project show/create/edit

**Files:**
- Modify: `apps/web/src/resources/project/project-show.tsx`
- Modify: `apps/web/src/resources/project/project-create.tsx` (`ProjectFormFields`, shared by edit)
- Test: `apps/web/src/resources/project/project-show.test.tsx` (create/extend) + a form test

**Interfaces:**
- Consumes: `ProjectGeofencesMap` (Task 6), `useRecordContext`/`useWatch`.
- Produces: `ProjectShowMap` (reads `useRecordContext().geofences`) and `ProjectFormMap` (reads `useWatch({ name: "geofences" })`) thin wrappers (may live in `project-geofences-map.tsx` or the resource files — keep them next to their use).

- [ ] **Step 1 — Failing test.** Mock `ProjectGeofencesMap` to a stub that renders its `ids` (e.g. `<div data-testid="pmap">{ids.join(",")}</div>`). For show: render `ProjectShow` with a record `{ geofences: [3,4] }` → assert the stub shows `3,4`. For the form: render `ProjectFormFields` inside a `Form` with `defaultValues:{ geofences:[5] }` → assert the stub shows `5`; then (if feasible) change the field and assert reactivity, else assert the initial wiring only and note it.
- [ ] **Step 2 — Run, verify it fails.** `cd apps/web && bun run test project-show project` → FAIL.
- [ ] **Step 3 — Implement.** Add `ProjectShowMap` to project-show (keep the existing chips), `ProjectFormMap` to `ProjectFormFields` below the geofences `ReferenceArrayInput`.
- [ ] **Step 4 — Run, verify pass.** Typecheck.
- [ ] **Step 5 — Commit.** `feat(web): project show/create/edit render the member-geofence map`

---

### Task 8: Geofence-show neighbor toggle (extend `DeckGeoJsonField`)

**Files:**
- Modify: `apps/web/src/components/deck/deck-geojson-field.tsx` (add generic `extraLayers` / `getTooltip` / extra-control-slot props)
- Modify: `apps/web/src/resources/geofence/geofence-show.tsx` (`FenceMapField` uses `useNeighborOverlay`)
- Test: `apps/web/src/components/deck/deck-geojson-field.browser.test.tsx` (extend) + `geofence-show.browser.test.tsx` (extend)

**Interfaces:**
- Consumes: `useNeighborOverlay` (Task 5).
- Produces on `DeckGeoJsonField`: `extraLayers?: Layer[]` (appended after geojson + marker layers), `getTooltip?` (passed to DeckMap), and a way to render extra control buttons — add `extraControls?: ReactNode` rendered as a DeckMap child next to the existing marker toggle. All optional/back-compat.

- [ ] **Step 1 — Failing test.** (deck-geojson-field) render with `extraLayers={[aLayer]}` + `extraControls={<button>Show Neighbors</button>}` → assert the button appears; without them → absent (back-compat). (geofence-show) `FenceMapField` for a record `{ mode:"quest", geometry:<polygon>, id:1 }` (mock `useNeighborOverlay` to expose a toggle) → assert a "Show Neighbors" button appears; clicking calls `setOn`.
- [ ] **Step 2 — Run, verify it fails.** `cd apps/web && bun run test:browser deck-geojson-field geofence-show` → FAIL.
- [ ] **Step 3 — Implement.** Add the three optional props to `DeckGeoJsonField` (append `extraLayers`, pass `getTooltip`, render `extraControls`). In `FenceMapField`: `const nb = useNeighborOverlay(record?.geometry, record?.id)`; pass `extraLayers={nb.layers}`, `getTooltip={nb.getTooltip}`, `extraControls={<Button ...>{nb.on ? "Hide Neighbors" : "Show Neighbors"}</Button>}` (position it below the marker toggle, e.g. stack in a top-left column).
- [ ] **Step 4 — Run, verify pass.** Typecheck.
- [ ] **Step 5 — Commit.** `feat(web): geofence-show ghost-neighbor toggle`

---

### Task 9: Geofence edit/create neighbor toggle (`GeofenceMap`)

**Files:**
- Modify: `apps/web/src/components/deck/geofence-map.tsx` (add `show.neighbors` to the toggle row + wire the overlay)
- Test: `apps/web/src/components/deck/geofence-map.browser.test.tsx` (extend)

**Interfaces:**
- Consumes: `useNeighborOverlay` (Task 5), the existing `show` state + toggle-row pattern (geofence-map.tsx:33-38, 104-115), `useWatch({ name:"geometry" })` (already present), the editing fence id.

- [ ] **Step 1 — Failing test.** Render `GeofenceMap` inside an RHF `Form` with a `geometry` polygon → assert a "Neighbors" toggle button appears in the control row; toggling it on (mock `useNeighborOverlay` / `useGeofencesByBbox`) adds the overlay layer. Assert the map still renders (no crash) with neighbors off (back-compat).
- [ ] **Step 2 — Run, verify it fails.** `cd apps/web && bun run test:browser geofence-map` → FAIL.
- [ ] **Step 3 — Implement.** `const nb = useNeighborOverlay(geometry, editingId)` where `editingId` = the record id on edit (read via `useRecordContext`/route; on create it's undefined). Add a `neighbors` entry to the `show` state + a 5th `<Button>` in the toggle row wired to `nb.setOn`/`nb.on`. Merge `nb.layers` into the layer array passed to `DeckGeoJsonInput`/`buildBaseLayers`, and pass `nb.getTooltip` to the underlying `DeckMap` (thread through `DeckGeoJsonInput` if it doesn't already accept `getTooltip` — add the passthrough if needed).
- [ ] **Step 4 — Run, verify pass.** Then run BOTH full suites once: `cd apps/web && bun run test` and `bun run test:browser`, and `cargo test -p koji-service` (report counts). Typecheck.
- [ ] **Step 5 — Commit.** `feat(web): geofence edit/create ghost-neighbor toggle`

---

## Preview verification (after Task 9)

In Claude Preview (dev server via `.claude/launch.json`; a live koji-server + data may be unavailable — note if so):
- Project show/create/edit: member fences render; hovering shows a name; clicking opens the fence's edit page in a new tab; editing the create/edit geofence list updates the map.
- Geofence show/edit/create: "Show Neighbors" reveals dimmed ghost fences; hover=name; click=new-tab edit; toggle off hides them.

## Follow-up (spin off after merge)

- Chip: replace the in-memory `?bbox=` filter with DB bbox columns (`min_lat/max_lat/min_lng/max_lng`, backfilled on save) + index for SQL-level narrowing, when fence counts grow past a few thousand.

## Self-Review (done)

- **Spec coverage:** backend `?ids=`→T1, `?bbox=`→T1; getTooltip→T2; overlay layer + new-tab open→T3; data hooks→T4; useNeighborOverlay→T5; project map→T6+T7; geofence-show neighbor→T8; geofence edit/create neighbor→T9. ✅
- **Placeholder scan:** no TBD/TODO; each task carries signatures + reuse pointers. ✅
- **Type consistency:** `getTooltip: (info)=>({text}|null)` consistent T2/T3/T5/T6/T8/T9; `geofenceOverlayLayer` opts shape consistent T3/T5/T6; `useGeofencesBy*` shapes consistent T4/T5/T6; Bounds order `[minLng,minLat,maxLng,maxLat]` consistent with the `?bbox=` param. ✅
