# Route / Geofence view feedback — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Address 7 tester-feedback items on the v2 route/geofence view (radius circles, header clarity, expand-to-viewport, geofence outline, mode-driven marker toggle, route stats) — all frontend, zero DB/API changes.

**Architecture:** The read-only show map is the edit workbench (`RouteMap`) minus the form + calc controls. A new `RouteShowMap` reads the route via `useRecordContext`, fetches the parent geofence, and reuses `buildBaseLayers` / `useMarkers` / `RouteStatsPanel`. A shared `useMarkerOverlay` hook gives both show pages the mode-driven marker toggle; `DeckMap` gains an `expandable` viewport toggle.

**Tech Stack:** React 19, react-admin / shadmin-core, deck.gl v9 + react-map-gl/maplibre, TanStack Query (via `useMarkers`), Vitest browser provider, Tailwind v4, bun.

**Spec:** `docs/superpowers/specs/2026-07-13-route-view-feedback-design.md`

## Global Constraints

- Frontend-only. No DB migration, no API endpoints, no route-model columns.
- No changes to `RouteMap`/`GeofenceMap` edit workbenches or the `/map` playground.
- Package manager: **bun** (`bun install`, `bun run test`). Commit `bun.lock` if deps change (none expected).
- Modes are `{unset, pokemon, fort, quest}`. Markers per mode via `routeModeMarkerCategories`: pokemon→[spawnpoint], quest→[pokestop], fort→[gym, station, pokestop].
- Read persisted calc params from the existing global key `CALC_PERSIST_KEY = "koji.map.calc"`; never call `calc.run` on show pages.
- deck.gl layer visuals (coverage circles, fence outline, stats populate) are verified in **Claude Preview**, not asserted in unit tests (brittle). Unit tests cover observable DOM: buttons, labels, testids, toggle default state.
- Run the vitest browser suite **once at each task's end**, not per step. Background any run >5s.

---

### Task 1: `DeckMap` expand-to-viewport toggle

**Files:**
- Modify: `apps/web/src/components/deck/deck-map.tsx` (props at :40-50, container at :112-118)
- Test: `apps/web/src/components/deck/deck-map.test.tsx` (create if absent)

**Interfaces:**
- Produces: `DeckMapProps.expandable?: boolean` (default `false`). When true, an expand button renders top-right; clicking swaps the container to a viewport-filling overlay; Esc collapses.

- [ ] **Step 1 — Failing test.** Render `<DeckMap layers={[]} expandable />`. Assert: a button with an accessible name like `/expand|full/i` exists; the `[data-testid="deck-map"]` container does **not** have `fixed` in its class. Click the button → container class now includes `fixed inset-0`. Press `Escape` → `fixed` gone again. Also render `<DeckMap layers={[]} />` (no prop) and assert no expand button.
- [ ] **Step 2 — Run, verify it fails** (button/behavior absent). `cd apps/web && bun run test deck-map`.
- [ ] **Step 3 — Implement.** Add `expandable = false` to props. Add `const [expanded, setExpanded] = useState(false)`. Container className: keep current classes when collapsed; when `expanded`, use `fixed inset-0 z-50 rounded-none` and force full height (override the inline `style={{height}}` with `height: "100%"` while expanded). Render, only when `expandable`, a small shadcn/icon button positioned `absolute right-2 top-2 z-20` (place it as a sibling to `{children}` inside the container so it sits above the canvas) that toggles `expanded`; label "Expand"/"Collapse" via `aria-label`. Add a `useEffect` that, while `expanded`, attaches a `keydown` listener collapsing on `Escape` (cleanup on unmount/collapse). The existing `ResizeObserver` already updates `sizeRef` when the container grows, and deck.gl/MapLibre auto-resize their canvas — no manual refit.
- [ ] **Step 4 — Run, verify pass.**
- [ ] **Step 5 — Commit.** `feat(web): add expand-to-viewport toggle to DeckMap`

---

### Task 2: `useMarkerOverlay` hook

**Files:**
- Create: `apps/web/src/components/deck/use-marker-overlay.ts`
- Test: `apps/web/src/components/deck/use-marker-overlay.test.tsx`

**Interfaces:**
- Consumes: `useMarkers` (`@/map/data/use-markers`), `routeModeMarkerCategories` (`./route-mode`), `geometryBounds` (`./bounds`), `COLOR` (`@/map/lib/map-colors`), `MarkerCategory`/`Bounds` (`@/map/stores/types`), deck `ScatterplotLayer`.
- Produces:
  ```ts
  export function useMarkerOverlay(
    mode: string | undefined,
    area: GeoJSON.Geometry | null | undefined,
    opts?: { alwaysFetch?: boolean },   // route-show passes true so stats can read data while toggle is off
  ): {
    on: boolean;
    setOn: (v: boolean) => void;
    label: string;                       // e.g. "Spawnpoints" / "Forts" — for the toggle button
    markerLayers: Layer[];               // built only when `on`; spread into the consumer's layer array
    data: Partial<Record<MarkerCategory, [number, number][]>>; // per-category points (for stats)
  }
  ```
  Category radii/colors mirror `RouteMap`'s markerSets: gym `COLOR.gym` r70/max12, pokestop `COLOR.pokestop` r40/max6, spawnpoint `COLOR.spawnpoint` r12/max2, station `COLOR.station` r40/max6 (`radiusUnits:"meters"`).

- [ ] **Step 1 — Failing test.** With a MSW/mock for `useMarkers` (or the app's existing test data provider), render a probe component using `useMarkerOverlay("pokemon", <a polygon>)`. Assert default `on === false` and `markerLayers.length === 0`. Call `setOn(true)` → after fetch resolves, `markerLayers.length >= 1` and the layer id references `spawnpoint`. Assert `label === "Spawnpoints"`. Repeat with `mode="quest"` → label "Pokestops", layer references `pokestop`.
- [ ] **Step 2 — Run, verify it fails** (module missing). `cd apps/web && bun run test use-marker-overlay`.
- [ ] **Step 3 — Implement.** `cats = routeModeMarkerCategories(mode)`. `bbox = area ? geometryBounds(area) ?? WORLD : WORLD`. For each of the 4 categories call `useMarkers(cat, area ?? null, bbox, 0, (opts?.alwaysFetch || on) && cats.includes(cat))` (fixed hook order — call all four unconditionally). Build `data` from the enabled categories. When `on`, build one `ScatterplotLayer` per non-empty enabled category (styling above); else `markerLayers = []`. `label` = human name of the category set: single-category modes use that name ("Spawnpoints"/"Pokestops"); fort → "Forts". Keep `WORLD` local const `[-180,-85,180,85]`.
- [ ] **Step 4 — Run, verify pass.**
- [ ] **Step 5 — Commit.** `feat(web): add useMarkerOverlay hook for mode-driven show-page markers`

---

### Task 3: `RouteShowMap` — read-only route map

**Files:**
- Create: `apps/web/src/components/deck/route-show-map.tsx`
- Test: `apps/web/src/components/deck/route-show-map.test.tsx`

**Interfaces:**
- Consumes: `useRecordContext` (shadmin-core), `useGetOne` (shadmin-core), `useCalc`/`CALC_PERSIST_KEY` (`./use-calc`), `useMarkerOverlay` (Task 2), `buildBaseLayers` (`@/map/lib/layers`), `routeCoordsToClusters` (`@/map/lib/calc-request`), `geometryBounds` (`./bounds`), `RouteStatsPanel` (`./route-stats-panel`), `DeckMap` (Task 1), `COLOR`.
- Produces: `export function RouteShowMap(): JSX.Element` — no props; reads the route from record context. Renders one `DeckMap` (height 640, `expandable`) containing the fence outline + route path + per-point coverage circles + optional markers + a marker toggle button + `RouteStatsPanel`.

Reuse map (mirror `RouteMap`, dropping the form):
- Fence fetch: mirror `route-map.tsx:41-56` but `geofenceId = record.geofence_id`.
- `geometry = record.geometry`; `routeMode = record.mode`.
- Radius: `const calc = useCalc(undefined, CALC_PERSIST_KEY)`; use `calc.params.radius` (fallback 70). Draw circles **regardless of strategy** (spec §A): pass `calcResultRadius: calc.params.radius ?? 70`.
- Layers: mirror `route-map.tsx:181-264` with `displayResult = routeFC` (the record geometry wrapped as a FeatureCollection), `calcResultIsRoute: true`, `geofences: true` + `geofences: fenceFC`, `markerSets` fed from `overlay.data`, visibility gated on `overlay.on`. Spread `overlay.markerLayers` OR feed the marker sets through `buildBaseLayers` visibility (pick one; don't double-render).
- Overlay: `const overlay = useMarkerOverlay(routeMode, fence?.geometry, { alwaysFetch: true })` — always fetch so stats have points; toggle only controls display.
- Stats: mirror `route-map.tsx:74-78,135-163,177-179` with a throwaway `useCalc()` instance; `dataPoints` = union of `overlay.data` categories; `clusters = routeCoordsToClusters(routeFC)`; auto-run on 600ms debounce. Feed `<RouteStatsPanel stats={statsCalc.stats} loading={...} />`.
- Fit bounds: `geometryBounds(geometry)` if present, else the fence (spec §D).
- Marker toggle button: render inside `DeckMap` (absolute, e.g. `left-2 top-2 z-10`), label `overlay.on ? "Hide {label}" : "Show {label}"`, `onClick` → `overlay.setOn(!overlay.on)`.

- [ ] **Step 1 — Failing test.** Render `<RouteShowMap>` inside a `RecordContextProvider` with a route record `{ geofence_id, mode: "pokemon", geometry: <MultiPoint> }` and the test data provider stubbing the geofence get + golbat markers. Assert: the `deck-map` testid renders (not the empty placeholder); a "Show Spawnpoints" toggle button exists; the `RouteStatsPanel` root (its testid/heading) is present. Click the toggle → label flips to "Hide Spawnpoints".
- [ ] **Step 2 — Run, verify it fails.** `cd apps/web && bun run test route-show-map`.
- [ ] **Step 3 — Implement** per the reuse map above.
- [ ] **Step 4 — Run, verify pass.**
- [ ] **Step 5 — Commit.** `feat(web): add read-only RouteShowMap (fence outline, coverage circles, markers, stats)`

---

### Task 4: Route-show header labels + swap map

**Files:**
- Modify: `apps/web/src/resources/route/route-show.tsx` (whole file, currently :1-18)
- Test: `apps/web/src/resources/route/route-show.test.tsx` (create if absent)

**Interfaces:**
- Consumes: `RouteShowMap` (Task 3), `SelectField` (react-admin/shadmin), `ROUTE_MODES` (`@/lib/constants`).

- [ ] **Step 1 — Failing test.** Render `RouteShow` (via the app's resource test harness / `AdminContext`) for a record `{ name: "Rijen_mon", mode: "pokemon", ... }`. Assert: the string "Rijen_mon" appears **at most once** in the body region (title/breadcrumb chrome excluded, or assert the body `name` TextField is absent); the mode renders as "Pokémon" (friendly), not "pokemon"; visible labels "Mode"/"Description" exist.
- [ ] **Step 2 — Run, verify it fails.** `cd apps/web && bun run test route-show`.
- [ ] **Step 3 — Implement.** Remove `<TextField source="name" />`. Replace `<TextField source="mode" />` with `<SelectField source="mode" choices={ROUTE_MODES} label="Mode" />`. Add `label` to the description/geofence fields (match `geofence-show.tsx`'s field-labeling pattern for consistency — read it first). Replace `<DeckGeoJsonField source="geometry" variant="route" height={400} />` with `<RouteShowMap />`.
- [ ] **Step 4 — Run, verify pass.**
- [ ] **Step 5 — Commit.** `fix(web): clarify route-show header (friendly mode, drop duplicate name, labels)`

---

### Task 5: `DeckGeoJsonField` optional markers + expandable + taller default

**Files:**
- Modify: `apps/web/src/components/deck/deck-geojson-field.tsx` (props :12-21, body :26-68)
- Test: `apps/web/src/components/deck/deck-geojson-field.test.tsx` (create if absent)

**Interfaces:**
- Consumes: `useMarkerOverlay` (Task 2), `DeckMap.expandable` (Task 1).
- Produces: new optional props `markerMode?: string`, `markerArea?: GeoJSON.Geometry | null`, `expandable?: boolean`. Default `height` becomes **640** (was 400). When `markerMode` + `markerArea` are set, the field runs `useMarkerOverlay(markerMode, markerArea)` (default `alwaysFetch:false`), merges `markerLayers` into its layer array, and renders a toggle button (mirroring Task 3's) as a `DeckMap` child.

- [ ] **Step 1 — Failing test.** Render `<DeckGeoJsonField source="geometry" markerMode="pokemon" markerArea={<polygon>} expandable />` inside a record context whose `geometry` is a polygon, with markers stubbed. Assert: a "Show Spawnpoints" toggle exists (default off → no marker layer / assert via a spy or that toggling adds a scatterplot layer id); an expand button exists (from `expandable`). Render without `markerMode` → no toggle button (back-compat). Assert the default height is 640 when `height` omitted (inspect the `[data-testid="deck-map"]` inline style).
- [ ] **Step 2 — Run, verify it fails.** `cd apps/web && bun run test deck-geojson-field`.
- [ ] **Step 3 — Implement.** Add the three props; `height = 640` default. When `markerMode && markerArea`, call `useMarkerOverlay` (hook order: call it unconditionally at top with possibly-null area; it must tolerate null). Concatenate `overlay.markerLayers` after the geojson layers. Pass `expandable` through to `DeckMap`. Render the toggle button + pass it (and nothing else) as `DeckMap` children only when `markerMode` set. Keep the empty-geometry early return, but note: if `markerMode` is set the field should still render the map even when the record geometry is a valid polygon (geofence-show always has geometry, so the early return isn't hit).
- [ ] **Step 4 — Run, verify pass.**
- [ ] **Step 5 — Commit.** `feat(web): DeckGeoJsonField optional marker overlay + expandable + taller default`

---

### Task 6: Wire geofence-show markers + expand

**Files:**
- Modify: `apps/web/src/resources/geofence/geofence-show.tsx` (:42 area)
- Test: extend `apps/web/src/resources/geofence/geofence-show.test.tsx` if one exists; else a minimal render test.

**Interfaces:**
- Consumes: the Task 5 props on `DeckGeoJsonField`.

- [ ] **Step 1 — Failing test.** Render `GeofenceShow` for a record `{ mode: "quest", geometry: <polygon> }` with markers stubbed. Assert a "Show Pokestops" toggle button appears and an expand button appears.
- [ ] **Step 2 — Run, verify it fails.** `cd apps/web && bun run test geofence-show`.
- [ ] **Step 3 — Implement.** On the geofence-show `DeckGeoJsonField`, pass `markerMode={record.mode}` `markerArea={record.geometry}` `expandable`. Use `useRecordContext` to read the record if not already available at that call site (the field is inside a record context; read `mode`/`geometry` via a tiny wrapper or `useRecordContext` in the show component).
- [ ] **Step 4 — Run, verify pass.** Then run the full `apps/web` browser suite once (background it): `cd apps/web && bun run test`.
- [ ] **Step 5 — Commit.** `feat(web): show mode-driven markers + expand toggle on geofence-show`

---

## Preview verification (after Task 6)

In Claude Preview (start the web dev server via `.claude/launch.json`; mind the `document.hidden` trap — the stats debounce is a plain `setTimeout`, unaffected):
- Route-show: coverage circles visible at the drawer radius; parent geofence outlined; markers toggle works; stats panel populates without a recalc; expand fills the viewport, Esc collapses; header reads "Route <name>" + labeled Mode "Pokémon" with no duplicate name.
- Geofence-show: markers toggle for the fence's mode; expand works.

## Self-Review (done)

- **Spec coverage:** A→T3, B→T4, C→T1(+T5 default height/expandable, T4/T6 wire), D→T3, E→T2+T3+T5+T6, F→T3. Basemap intentionally omitted. ✅
- **Placeholder scan:** no TBD/TODO; reuse pointers give exact `file:line` sources. ✅
- **Type consistency:** `useMarkerOverlay` return shape (`on`/`setOn`/`label`/`markerLayers`/`data`) used identically in T3/T5; `expandable` prop name consistent T1/T5/T6; `CALC_PERSIST_KEY` reused not redefined. ✅
