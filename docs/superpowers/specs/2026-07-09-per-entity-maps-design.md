# Per-Entity Maps — Design Spec

**Date:** 2026-07-09
**Branch:** claude/v2
**Status:** Approved (delegate mode)

## 1. Context & Problem

The v2 interactive map is a fresh deck.gl build at `apps/web/src/map/**`, mounted as a
full-bleed react-admin `CustomRoute` at `/map`. It grew through Phases 1–3 into a capable
but **monolithic god-map**: one `DeckCanvas` singleton wired to four global zustand stores
(`map-view`, `map-ui`, `map-calc`, `map-settings`) plus "fetch every geofence / route /
marker" query hooks, with all drawing, editing, filtering, and calc funneled through that
single overworld surface.

That is the same shape as v1's map — and, per the owner, "a logistical and UI/UX nightmare."
Managing every area and every route from one canvas does not scale: you cannot focus on one
fence, the state is global, and there is no per-entity workflow.

Meanwhile the per-entity resource pages are thin: `geofence-show` renders a 400px **read-only**
Leaflet `GeoJsonField`; `geofence-edit` has **no geometry editing at all**; `route-show` is a
read-only Leaflet `MultiPointField`; `route-edit` is a plain form. The powerful deck surface and
the per-entity pages are disjoint.

Separately, the owner wants these deck components built **reusable enough to eventually replace
shadmin's Leaflet suite** (`apps/web/src/components/leaflet/**`), which is a full field/input
pattern library (`<XField source/>` read-only + `<XInput source/>` RHF-controlled, per geometry
type, plus geojson / feature / feature-collection, geocoding, OSM import).

## 2. Goals

- Replace the overworld god-map with **focused, per-entity map surfaces**: one map per area
  (geofence), one map per route.
- Build the deck components as **record-driven, reusable** units that mirror shadmin's Leaflet
  **field/input contract**, so they are a drop-in path to replace the Leaflet suite later.
- Add **geometry editing** to geofences (currently impossible on the edit page).
- Make **routes first-class calc results**: the route edit page becomes a calc workbench bound
  to its parent geofence, finally wiring the #1 backlog item (calc → save-as-route).
- No backend changes.

## 3. Non-Goals (this round)

- Not porting all ~40 Leaflet shape components — only the polymorphic field + geojson input +
  the two entity workbenches Koji actually uses. The contract is designed so the rest follow.
- Not deleting or rewriting the Leaflet suite — the deck suite grows **alongside** it; the
  backport/replacement is a later, separate effort once the deck contract is proven.
- Not touching OSM import / geocoding search (stay Leaflet; off the per-entity critical path).
- No backend / migration / API changes.

## 4. Architecture

New home: **`apps/web/src/components/deck/`** — a reusable sibling to `components/leaflet/`.
`src/map/` remains the (soon-thin) app route that consumes it; its pure helpers
(`lib/coords.ts`, `lib/edit-modes.ts`, `lib/edit-serialize.ts`, `lib/layers.ts`,
`lib/calc-overlay.ts`, `lib/calc-request.ts`, `data/*` hooks) are reused by the deck suite.

Layers, bottom-up:

### L0 — `<DeckMap>` primitive (`components/deck/deck-map.tsx`)

The store-free base. Owns the deck root (interaction controller), the MapLibre child, and a
**transient per-instance camera** (deck `initialViewState` + a local `onViewStateChange`
callback — **no global store**). This is the deck analog of Leaflet's `<BaseMap>` and the core
reusability unlock: instances are fully independent, N maps coexist on a page, and there is no
singleton to desync (which also kills the HMR-desync class documented in Phase 2).

Props:
- `layers: Layer[]` — deck layers to render.
- `fitBounds?: GeoJSON geometry | Bounds` — auto-fit on mount (mutually exclusive with…).
- `initialViewState?: ViewState`.
- `height?: number | string` (default 400).
- `tileUrl?: string` (default `DEFAULT_TILE_URL`).
- `onViewStateChange?: (vs, bounds) => void` — optional; workbenches that fetch by bounds use it.
- `getCursor?`, `controller?` overrides.
- `children` — absolutely-positioned overlay panels (toolbars, calc controls, readouts).

### L1 — `<DeckGeoJsonField>` (read-only field; `components/deck/deck-geojson-field.tsx`)

Reads `record[source]` via `useRecordContext`, renders one GeoJSON geometry on a `<DeckMap>`,
auto-fits to it. **Drop-in for both Leaflet `GeoJsonField` and `MultiPointField`** — one
polymorphic field (deck's `GeoJsonLayer` handles Point / Line / Polygon / Multi* /
GeometryCollection). Prop surface mirrors `BaseFieldProps`:
`{ source, height, tileUrl, pathOptions, fitBounds, emptyText, zoom? }`.

Route styling: when the geometry is an ordered route (MultiPoint / LineString), the field can
draw the connecting path + per-segment "long jump" coloring by reusing `calc-overlay.ts`
(`routeSegments`). Controlled by an optional `variant?: "geometry" | "route"` prop
(default `"geometry"`).

### L2 — `<DeckGeoJsonInput>` + `useDeckEditRHF` (editable input)

`useDeckEditRHF({ source, ... })` (`components/deck/use-deck-edit-rhf.ts`) is the deck analog of
`useGeomanRHF`: it watches `source` via RHF `useWatch`, hydrates draft features from the form
value, drives `@deck.gl-community/editable-layers` edit modes, and commits geometry back to the
form via `setValue` with the same echo-dedup discipline (`lastWrittenValue` ref). It reuses the
existing pure `edit-modes.ts` (`modeSpecFor`) and `edit-serialize.ts` (`shouldCommitEdit`,
`explodeSplitFeatures`).

`<DeckGeoJsonInput>` (`components/deck/deck-geojson-input.tsx`) composes a draw/edit toolbar
(reusing `DrawToolbar`'s buttons, but driven by **local** hook state, not the global ui-store) +
`<DeckMap>` + `useDeckEditRHF`. Prop surface mirrors `BaseInputProps`:
`{ source, label, helperText, disabled, validate, pathOptions, height, tileUrl, shapes? }`.
Drop-in for Leaflet `GeoJsonInput`. This is a **new capability**: geofence geometry editing.

### L3 — Per-entity workbenches

**`<GeofenceMap>`** (`components/deck/geofence-map.tsx`) — the per-area surface:
- `<DeckGeoJsonInput source="geometry">` to edit the fence polygon.
- Markers + S2 cells scoped to the fence's **client-computed `@turf/bbox`** (feed the bbox to the
  existing `useMarkers` / `useS2Cells` — no backend change), as read-only context.
- Local layer toggles / last-seen filter (per-instance, not global).
- No calc here (calc moved to route-edit — see §5).

**`<RouteMap>`** (`components/deck/route-map.tsx`) — the per-route calc workbench (see §5).

### L4 — Retire the god-map

`/map` collapses to a **read-only index**: render all fences (reuse `useGeoFeatures("geofences")`),
click a fence → navigate to its edit page / workbench. No editing / calc / drawing at the
overworld level. Keep it (recommended) rather than delete — spatial "where are my fences"
navigation is the one thing an overworld is genuinely good at. The existing `/map` route stays
live until L1–L3 land, then flips to the index.

## 5. Route Edit = Calc Workbench (bound to `geofence_id`)

Ground truth: `crates/koji-db/src/db/route.rs` — `geofence_id: u32` is **mandatory** (every route
belongs to exactly one fence). Calc taxonomy (`calc-request.ts`): `AREA_MODES` = `[cluster, route,
bootstrap]` compute over an **area** (fence polygon) + golbat category; `ROUTE_INPUT_MODES` =
`[reroute, routeStats]` re-process an existing route's points. Today calc → save-as-route is **not
wired** (overlay-only); the only path a route reaches the DB is the manual `MultiPointInput` or
import.

The route edit page becomes the calc workbench:

1. **Reactive area load.** `useWatch({ name: "geofence_id" })` → `getOne("geofence", id)` → render
   that fence's geometry as a **read-only area layer** *and* feed it as the calc **area input**.
   Changing the geofence dropdown re-loads the area live (same `useWatch`-drives-UI pattern used
   for the webhook mode-conditional fields).
2. **Calc controls on the page.** The full Phase-3 calc surface (mode / category / algorithm /
   radius / S2 / min-points / sort), scoped to *this* route's fence:
   - Fresh route (no geometry): AREA_MODES over the loaded fence → generate points.
   - Existing route: ROUTE_INPUT_MODES over its own points → re-optimize; or re-run AREA_MODES to
     regenerate.
3. **Result → the route's `geometry`.** Run calc (`submitCalc` → `POST /api/v2/jobs` → poll /
   subscribe `jobs/{id}`) → preview the result overlay → the ordered points become the form's
   `geometry` value (RHF `setValue`) → **Save** persists the route via `create` / `update`.
   The calc `category` maps to the route `mode` (v1: "using the category to auto set the mode").
4. **Fence is context, not editable here.** You edit the polygon on the geofence page; on the
   route page it is a read-only backdrop + the calc area.
5. **Manual fallback stays.** Hand-drawing points (`DeckGeoJsonInput` / Leaflet `MultiPointInput`
   + fence picker) remains available for the rare manual edit.

The calc panel is extracted from its global stores into a **headless `useCalc()` hook** (local
state, mirroring the `map-calc-store` shape) + a presentational `<CalcControls>` component. The
route workbench mounts `<CalcControls>` with area = the watched fence and an `onResult` callback
that writes `geometry`. The global `map-calc-store` remains only for the soon-retired overworld.

### Fence → routes linkage

Geofence show/edit gains a routes list: `ReferenceManyField(target="geofence_id",
reference="route")` (same pattern as the project→webhooks section) + a **"New route" button** that
pre-fills `geofence_id` and jumps to route-create/edit. Click a route → its `<RouteMap>`. The
spine reads top-down: **fence defines the area → route picks a fence + calcs over it → save.**

## 6. State Strategy

The four global zustand stores were built for one singleton map. Per-instance maps cannot share
global singletons (two workbenches would fight). Therefore:

- **Camera** → transient per-instance (deck `initialViewState` + local `onViewStateChange`). Drop
  `map-view-store` for embedded maps.
- **UI / draw / calc** → per-instance: local `useState` / `useReducer` (+ React context only if a
  tree gets deep). `useDeckEditRHF` holds draft/selection locally; `useCalc()` holds calc state
  locally. **Ponytail:** start with local state; escalate to a per-instance store factory only if
  prop-drilling actually hurts.
- **Settings** (`tileServerId`, `markerRadius`) → genuinely global user prefs → keep
  `map-settings-store` as a real singleton.

## 7. Data Flow

- **Fence bbox → markers/S2:** `@turf/bbox(fence.geometry)` → `Bounds` → `useMarkers(cat, bounds,
  …)` / `useS2Cells(level, bounds, …)`. Already-present dep; no backend change.
- **`geofence_id` → calc area:** watched id → `getOne("geofence")` → `featureToAreaFC(feature)` →
  calc body `area`.
- **calc result → route geometry:** `parseCalcResult(job)` → FC → ordered `[lon,lat]` points →
  route `geometry` (MultiPoint) via RHF `setValue`.
- **Coord discipline (unchanged, load-bearing):** golbat markers arrive `[lat,lon]` → transpose to
  `[lng,lat]` for deck (`coords.fromKojiLatLon` / `packMarkers`); geofence/route GeoJSON is already
  `[lng,lat]`; marker bbox wire is camelCase (`minLat`…), S2 `BoundsArg` is snake_case
  (`min_lat`…) — they differ.

## 8. Testing

deck WebGL renders offscreen-black in Claude Preview → verify by **DOM geometry + layer props +
network**, never `preview_screenshot` (documented trap). Per component:
- L0–L2: browser tests with `RecordContextProvider` (fields) and an RHF `<SimpleForm>` harness
  (inputs), asserting layer/prop wiring and form `setValue` — mirroring the Leaflet component tests.
- Pure helpers (`useCalc` reducer logic, category→mode mapping, bbox derivation) get jsdom unit
  tests.
- Workbenches: shell smoke tests with child hooks stubbed (lesson from Phase 2: stub hooks that
  need Admin / QueryClient context, else the shell smoke test breaks at the phase boundary).

## 9. Phasing (feeds writing-plans)

1. `<DeckMap>` + `<DeckGeoJsonField>` → swap into `geofence-show` & `route-show` (read-only, lowest
   risk, proves the record-driven contract end-to-end).
2. `useDeckEditRHF` + `<DeckGeoJsonInput>` → geometry editing in `geofence-edit`.
3. `<GeofenceMap>` workbench (markers/S2/route-list scoped to the fence) → geofence page.
4. Headless `useCalc()` + `<CalcControls>`; `<RouteMap>` calc workbench (reactive `geofence_id`
   area, AREA + ROUTE calc modes, result → geometry → save) → route pages.
5. Collapse `/map` → read-only index. Old map stays live until 1–4 replace it, then flip.

## 10. Assumptions (locked with owner)

1. New suite lives in `components/deck/`, mirroring `components/leaflet/` — reusability first.
2. `geofence-edit` = polygon editing + route list (no calc). **`route-edit` = the calc workbench**
   (auto-loads fence from `geofence_id`, reactive on change; runs AREA + ROUTE calc modes; result →
   route geometry → save). Manual `MultiPointInput` stays as fallback.
3. One polymorphic field/input, not 7 per-geometry-type components — build what Koji uses; per-type
   wrappers are trivial to add later for the full Leaflet-replacement backport.
4. No backend changes — bbox client-side (`@turf/bbox`), calc already fence-scoped.
5. Keep `/map` as a read-only index, don't delete.
6. Leaflet suite stays untouched this round; deck suite grows alongside.
7. Markers on show pages default off; the heavy marker + calc surface lives on edit/workbench.

## 11. Open Wire Details (resolve in the plan, not blockers)

- **`geofence_id` reference filter param:** backend route list uses `args.geofenceid`, but the
  generic `getManyReference` strips a trailing `_id` → `?geofence=`. Confirm the actual query param
  and special-case it if it is `geofenceid` (same class as the webhook `?project=` translation).
- **category → route `mode` mapping:** mirror v1's auto-set-from-category. Enumerate the exact
  `ROUTE_MODES` ↔ category map in the plan.
- **routing `sortBy` enum casing:** `geohash` vs `geoHash` ambiguity (deferred in Phase 3) still
  applies; keep the server default unless resolved.

## 12. Out of Scope

Leaflet-suite replacement beyond geofence/route; OSM import & geocoding; backend/migration changes;
save-result-to-golbat; overworld editing/calc; per-type deck shape components; bundle code-split of
the ~2.9MB deck chunk.
