# Route / Geofence view — tester feedback design

**Date:** 2026-07-13
**Branch:** claude/v2
**Status:** Approved (design), pending plan

## Context

An early tester reviewed the v2 **route view** (read-only `RouteShow` page) and filed 7 items.
Recon (8-agent workflow, `wf_65b234ac-e1c`) mapped each to current code. All fixes are
**frontend-only — zero DB/API changes** (the backend already serves every marker category,
and stats/radius are reused from existing machinery).

The dominant insight: `components/deck/route-map.tsx` (`RouteMap`, the create/edit calc
workbench) already renders almost everything the tester wants — parent-geofence outline,
mode-driven markers, the route path, per-point coverage circles at the persisted calc
radius, and an auto-run `RouteStatsPanel`. The read-only show map is `RouteMap` with the
form/calc-controls removed and the record read from `useRecordContext` instead of
`useFormContext`.

## Locked decisions (from Q&A, 2026-07-13)

| # | Item | Decision |
|---|------|----------|
| A | Radius circles | Read the saved calc radius from `localStorage["koji.map.calc"]` and draw coverage circles at it. No view control. Global value → same radius for every route (accepted). |
| — | Basemap | **Dropped.** Basemap is already wired (CartoDB); tester's meaning unclear, skipped. |
| C | Height / fullscreen | Expand-to-viewport toggle (CSS overlay, Esc/click to collapse — no Fullscreen API) on the shared `DeckMap`, + taller default height. Both show pages benefit. |
| E | Markers | Mode-driven overlay **toggle** (default OFF) on **both** show pages. Reuse existing backend + `useMarkers`. |
| D | Geofence outline | Route-show draws the parent geofence outline: outline-only, always-on, fit bounds to the **route** (not the fence). |
| F | Stats | Mount existing `RouteStatsPanel` on route-show, fed by the same auto-run background stats job the edit page uses. No schema change, always fresh, full grid. |
| B | Header | Route-show: drop the duplicate body `name` field, add field labels, render `mode` via the friendly `ROUTE_MODES` map. |

## Non-goals

- No DB migration, no new API endpoints, no route-model columns (radius/stats stay unpersisted).
- No basemap change.
- No changes to the edit/create workbenches (`RouteMap`, `GeofenceMap`) or `/map` playground.
- Geofence-show gets **only** the markers toggle + fullscreen (not the header/stats/outline work).

## Mode → category (verified in code)

`route-mode.ts` + backend `VALID_CATEGORIES = [gym, pokestop, spawnpoint, station, fort]`:

- `routeModeToCategory`: pokemon→spawnpoint, quest→pokestop, fort→fort, unset→pokestop.
- `routeModeMarkerCategories` (marker *rendering* set): pokemon→[spawnpoint], quest→[pokestop],
  **fort→[gym, station, pokestop]** (fort has no standalone marker layer, so it renders its constituents).

**Interpretation to confirm (⚠️):** the tester said "forts for forts". There is no standalone
`fort` marker rendering — `buildBaseLayers` only draws gym/pokestop/spawnpoint/station sets. So a
fort-mode route/fence will render its **constituent** markers (gyms + stations + stops) via
`routeModeMarkerCategories`, exactly as the edit workbench already does. pokemon/quest are
unaffected (both helpers agree). This also keeps show-page stats numbers identical to the edit page.

## Feature specs

### A. Radius coverage circles — route-show

- `RouteShowMap` reads persisted calc params via `useCalc(undefined, CALC_PERSIST_KEY)` (read-only —
  never calls `run`).
- Feed `buildBaseLayers({ calcResult: routeFC, calcResultIsRoute: true, calcResultRadius })` where
  `routeFC` wraps the route record's `geometry` (a `MultiPoint` of ordered stops) and
  `calcResultRadius = params.radius`.
- **Divergence from edit (intentional):** the edit page only draws circles when
  `strategy === "radius"`. The show page draws circles at `params.radius` **regardless of strategy** —
  the show page runs no calc, so "strategy" is irrelevant to it; the whole point of the feedback is to
  *see* the coverage radius, so we always show it. (If `params.radius` is somehow absent, fall back to
  the `DEFAULTS.radius` of 70.)

### B. Header labels + de-dup — route-show

- Remove `<TextField source="name" />` from the body (breadcrumb + "Route <name>" title already show it).
- Replace `<TextField source="mode" />` with `<SelectField source="mode" choices={ROUTE_MODES} />`
  (renders "Pokémon" not "pokemon"). Match whatever labeled-field wrapper `geofence-show.tsx` uses so
  the two show pages read consistently.
- Ensure Mode / Description / Geofence carry visible labels.

### C. Expand-to-viewport toggle + taller default — both show pages

- Add to `DeckMap` (shared container, `deck-map.tsx`):
  - New prop `expandable?: boolean` (default `false`; show maps pass `true`).
  - When `expandable`, render a small button top-right. Toggling sets an `expanded` state that swaps the
    container class to `fixed inset-0 z-50` (fills the browser viewport) and back.
  - A `keydown` Esc listener collapses when expanded.
  - deck.gl/MapLibre auto-resize their canvas on the container's `ResizeObserver` (already wired), so the
    map fills the enlarged container without a manual refit. Camera stays where the user left it.
- Default show-map height: **640px** (matches the edit workbench), replacing the current 400.

### D. Parent geofence outline — route-show

- `RouteShowMap` fetches the parent fence with `useGetOne("geofence", { id: geofence_id })` (mirrors
  `RouteMap:41-56`) — one fetch that serves both the outline **and** the marker query area (E).
- Render via `buildBaseLayers({ visibility: { geofences: true }, geofences: fenceFC })`.
- Outline-only, visually distinct from the route's orange path; always-on; **fit bounds to the route
  geometry** (`geometryBounds(routeGeometry)`), not the fence, so a small route in a large fence isn't
  zoomed out. If the route has no geometry yet, fall back to fitting the fence.

### E. Mode-driven marker overlay toggle — both show pages

- Shared hook `useMarkerOverlay(mode, areaPolygon)`:
  - Holds `on` state (default `false`) + a toggle.
  - Fetches the mode's marker categories (`routeModeMarkerCategories(mode)`) via `useMarkers`, scoped to
    `areaPolygon` + its bbox. Fetch is **enabled only when `on`** (read-only page: don't pay for golbat
    data until the user asks — except route-show, see note).
  - Returns `{ layers, on, setOn, label }` where `label` names the category (e.g. "Spawnpoints").
- A small toggle button (reuse an existing shadcn button; mirror `geofence-map.tsx:104-115`'s pattern).
- **route-show:** `mode = record.mode`, `areaPolygon = parent fence geometry` (from D's fetch).
- **geofence-show:** `mode = record.mode`, `areaPolygon = record.geometry` (its own polygon).
- No `LastSeenPicker`/tth on the read-only pages — plain snapshot at `lastSeen = 0`.
- **Stats caveat (route-show only):** stats coverage needs the golbat points regardless of the toggle.
  So on route-show the marker categories are fetched **always** (to feed the stats job, like `RouteMap`),
  and the toggle only controls whether the marker *layer* is added to the map. On geofence-show there is
  no stats job, so the fetch is gated on the toggle.

### F. Route quality stats — route-show

- Mount `RouteStatsPanel` inside `RouteShowMap`'s `DeckMap`, fed by a dedicated throwaway
  `useCalc()` instance running `runStats(...)` on a ~600ms debounce once geometry + marker points are
  available — copied from `RouteMap:74-78,135-163,312`.
- `dataPoints` = union of the mode's marker category results (same as `RouteMap:142-150`), so the
  numbers match the edit page. `clusters` = `routeCoordsToClusters(routeFC)`.
- Full `StatsGrid` (clusters, coverage %, distance, quality, score, …) — already built.

## Component structure (recommended)

- **`deck-map.tsx`** — add `expandable` prop + expand/Esc logic (shared; benefits every consumer that opts in).
- **`components/deck/use-marker-overlay.ts`** *(new)* — `useMarkerOverlay(mode, area, { alwaysFetch })` hook (E).
- **`components/deck/route-show-map.tsx`** *(new)* — read-only route map: `useRecordContext` → fence fetch
  (D) + route/circle layers (A) + `useMarkerOverlay` (E) + auto-run stats + `RouteStatsPanel` (F), rendered
  through one `DeckMap` at height 640 with `expandable`. Essentially `RouteMap` minus the form + `CalcControls`.
- **`resources/route/route-show.tsx`** — header/label edits (B); swap `DeckGeoJsonField` → `RouteShowMap`.
- **`resources/geofence/geofence-show.tsx`** — add the markers toggle + `expandable`. Either extend
  `DeckGeoJsonField` with optional `markerMode`/`markerArea` props (it already renders a `DeckMap`), or add a
  thin `GeofenceShowMap`. Prefer extending `DeckGeoJsonField` — it's the smaller diff and keeps one base
  polygon renderer. Bump its default height to 640 and thread `expandable`.

`RouteShowMap` and the geofence path both reuse `useMarkerOverlay`, so the toggle logic lives once.

## Testing

- Vitest browser-provider tests are the norm here; run per-task at task end, not per step.
- Cover: (A) circles render at the persisted radius; (B) `mode` shows friendly label + no duplicate name;
  (C) expand toggles the container class and Esc collapses; (D) fence outline layer present + bounds fit the
  route; (E) toggle off = no marker layer, on = layer present, correct category per mode; (F) stats panel
  populates from the auto-run job.
- Verify in Claude Preview (route-show + geofence-show) once wired — mind the `document.hidden` trap
  (offscreen preview pauses visibility-gated timers; the stats debounce is a plain `setTimeout`, unaffected).

## Risks / notes

- **Golbat fetch on every route-show view** (for stats) — same cost the edit page already pays; accepted per F.
- **Global radius** — every route shows the last-set drawer radius, not its own; accepted per A.
- **Fort marker interpretation** — renders constituents, flagged ⚠️ above for confirmation at spec review.
