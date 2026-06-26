# Koji v2 — Interactive deck.gl Map Page — Design Proposal

> Branch `claude/v2` · target app `apps/web` (react-admin / shadcn-admin-kit, Vite 8 + React 19 + TS 6 + Tailwind v4) · status: **brainstorming design doc, no code** · review mode: **delegate** (owner reviews §10 Assumptions, not a Q&A)

---

## 1. Overview & Goals

The Koji v2 map page is a **full-bleed, single-route WebGL map** mounted at `/map` as a react-admin `CustomRoute` inside `<Admin>` (inheriting `authProvider` + `dataProvider`, exactly like the existing `/import` wizard). It renders Koji's spatial domain — gym/pokestop/spawnpoint/station markers, geofences, routes, and S2 cell coverage — over a switchable raster base map, and lets the operator **draw/edit geofences**, **toggle and filter layers**, **launch and watch clustering/routing/bootstrap calc jobs**, and **import/export** geometry, all in one canvas. Scope is parity epics **10–14** (10: drawing/editing; 11: layers/markers/S2; 12: calc UI; 13: import/export; 14: map nav/popups/settings). It is a **clean deck.gl build** — zero reuse of v1 map code (mined only for parity feature-set, already captured), with **deck.gl v9** as the render core and **zustand** as the single source of UI/transient truth (the react-admin store is not type-safe enough for this surface).

---

## 2. Recommended Architecture

### Base-map model — **DECISIVE PICK: MapLibre GL JS as root, deck.gl as overlay**

Use **MapLibre GL JS** (raster XYZ style) as the React root map component via **react-map-gl/maplibre**, with deck.gl layers mounted as a **`MapboxOverlay` in non-interleaved (overlay) mode** through the `useControl` hook from `@deck.gl/mapbox`.

**Why this and not pure-deck-gl `TileLayer`:**
- MapLibre's native tile manager handles raster fetch/cache/LOD far better than driving every base tile through deck.gl's `TileLayer`. Koji's tile servers are pure XYZ raster (`{z}/{x}/{y}.png`) — **runtime tile-server switching is a `mapStyle` swap** (redefine the raster source in the style.json), no layer-data churn.
- It is the vis.gl-blessed, 2026-standard pattern; every official deck.gl React example uses it. React 19 compatible (compositional `useControl`, no hard peer pins).
- MapLibre owns the camera; deck.gl reads it. **One camera system, no two-camera sync.** zustand owns `viewState`, feeds MapLibre `initialViewState`/`onMove`, and deck.gl's overlay inherits the same view automatically.
- **Overlay mode (`interleaved: false`)** dodges the documented v9.3.0 interleaved regressions (spatial offset, GoogleMapsOverlay render crashes). Negligible visual lag for raster + markers on any modern GPU.
- Keeps the existing **leaflet + geoman form inputs fully isolated** — different rendering stack, same `authProvider`/`dataProvider`. Zero interference.

### Editing stack — **deck.gl-native: `@deck.gl-community/editable-layers` (EditableGeoJsonLayer)**

Geofence drawing/editing lives **in deck.gl**, not in a second map library. `@deck.gl-community/editable-layers` (the maintained nebula.gl successor — v9.3.7, published 2026-06, pins `@deck.gl/core@~9.3.0`, React 19 OK, pure ESM so Vite-clean) ships `EditableGeoJsonLayer` with all the modes Koji needs: `DrawPolygonMode`, `DrawRectangleMode`, `DrawCircleFromCenterMode`, `ModifyMode` (vertex edit), `TranslateMode`, `RotateMode`, `ScaleMode`, `SplitPolygonMode`, `DrawLineStringMode`/`DrawPointMode` (for routes). State is `{ data: FeatureCollection, mode, selectedFeatureIndexes, onEdit({updatedData, editType}) }` — which maps cleanly onto the zustand `draftFeatures`/`drawMode`/`selection` slice (the `onEdit` callback writes `updatedData` to the store). Polygon holes/cutouts → MultiPolygon are built-in. **Merge/union is NOT a built-in mode** → implement "merge selected polygons" with `@turf/union` + `@turf/difference` (turf already a dep). Edited features are plain GeoJSON `FeatureCollection`s → write back to the `geofence`/`route` resource via `dataProvider.update`. **No leaflet/geoman on the map page.** Caveat: editable-layers bundles turf v7 (~150–200 KB gz, partially offset since turf is already present).

### Layer set (deck.gl v9)

| Concern | Layer | Package |
|---|---|---|
| Markers (gym/stop/spawn/station, 10k–100k+) | `ScatterplotLayer` + `DataFilterExtension` (GPU time/category filter) | `@deck.gl/layers`, `@deck.gl/extensions` |
| Dense-marker heat (optional, 100k+) | `HexagonLayer` (world-space GPU agg) | `@deck.gl/aggregation-layers` |
| Geofences + routes (GeoJSON) | `GeoJsonLayer` (auto-dispatches Polygon/Path) | `@deck.gl/layers` |
| S2 cell coverage | `S2Layer` (token-string input) | `@deck.gl/geo-layers` |
| Editing | `EditableGeoJsonLayer` | `@deck.gl-community/editable-layers` |
| Calc results (clusters / routes) | `ScatterplotLayer` (clusters) + `PathLayer` (routes) | `@deck.gl/layers` |
| Selection highlight / hover | `GeoJsonLayer` (single-feature overlay) | `@deck.gl/layers` |

### Exact packages to add (pin majors; let `^` float minors)

```
maplibre-gl@^4.7.0
react-map-gl@^8.1.0            # /maplibre export path
@deck.gl/core@^9.3.0
@deck.gl/react@^9.3.0
@deck.gl/layers@^9.3.0
@deck.gl/geo-layers@^9.3.0
@deck.gl/aggregation-layers@^9.3.0
@deck.gl/extensions@^9.3.0
@deck.gl/mapbox@^9.3.0
@deck.gl-community/editable-layers@^9.3.7  # nebula.gl successor; pins @deck.gl/core ~9.3, React 19 OK
zustand@^5.0.8                 # confirm presence; add if absent
```
`luma.gl@^9` rides in transitively as a deck.gl peer — **do not pin it independently**; mismatched luma versions cause shader-compile failures. Import `maplibre-gl/dist/maplibre-gl.css` once in the map wrapper.

---

## 3. Two Alternatives Considered (the fork was not skipped)

**Alt B — Pure deck.gl `TileLayer` + `MapView` (drop MapLibre entirely).**
Smallest dependency footprint, single rendering system, total zustand control of the camera, no `mapStyle` indirection. **Lost because:** deck.gl's `TileLayer` re-implements raster tile management that MapLibre already does better (eviction, retina, LOD), tile-server switching becomes a manual `TileLayer.data` rebuild instead of a clean style swap, and you inherit multi-`View` complexity the moment any 3D/extruded geometry is wanted later. The footprint saving (~one dep) doesn't pay for the tile-pipeline work we'd re-own. Acceptable only for a tile-server-only shop; Koji isn't one.

**Alt C — Leaflet + `deck.gl-leaflet` bridge (reuse the geoman ecosystem).**
Tempting because the form inputs already use leaflet 1.9 + geoman, so the map page could share one library's camera and the geoman draw tooling. **Lost because:** `deck.gl-leaflet` is community-maintained (not vis.gl), it forces **two camera systems** (Leaflet + deck.gl) with sync glue, Leaflet's canvas/DOM rendering collapses past ~10k markers — the exact scale this redesign exists to fix — and the bridge lags the deck.gl+MapLibre combo on maintenance. A performance-first map cannot sit on Leaflet's renderer. We keep geoman **only** for the existing resource forms.

*(Editing-lib fork, recorded:* the runner-up to `EditableGeoJsonLayer` was keeping geoman-on-leaflet for drawing in a side panel. Rejected — it would reintroduce the two-stack split on the very page we're unifying. deck.gl-native editing keeps one canvas, one coordinate system, one GeoJSON round-trip.*)*

---

## 4. Zustand Store Design

Three small stores by update-frequency, plus react-query for server data. **The 60fps camera never goes through React state.**

### 4a. `mapViewStore` — camera, transient (NO React re-render at 60fps)

MapLibre/deck.gl run **uncontrolled** (`initialViewState` + `controller`/`onMove`). The live camera is written to a **ref**, and a **throttled "settled" snapshot** is published to the store for the *few* widgets that must react (bounds-fetch, coordinate readout). The high-frequency stream never triggers a subscription.

```ts
interface MapViewStore {
  // live camera — written ~60fps via store.setState WITHOUT any component selector subscribing to it.
  // Consumers read it transiently (store.getState() / subscribe-with-selector in a useEffect), never via useStore(s => s.live).
  liveViewState: ViewState;                 // {longitude, latitude, zoom, pitch, bearing}
  // throttled (~200ms) settled snapshot — THIS is what leaf widgets subscribe to.
  settledViewState: ViewState;
  settledBounds: [number, number, number, number]; // [minLng, minLat, maxLng, maxLat] (lng/lat order!)
  setLive: (v: ViewState) => void;          // hot path; updates liveViewState + schedules throttled settle
  // settle action computes bounds and writes settledViewState/settledBounds
}
```
*Mechanism, exactly:* deck.gl is **uncontrolled** — `onViewStateChange` calls `setLive` (a transient write). A `subscribeWithSelector` listener (registered once in `<DeckCanvas>`'s mount effect) throttles and writes `settledViewState`/`settledBounds`. Canvas frames cost **zero React renders**; only the handful of widgets selecting `settled*` re-render, and only ~5×/sec. We never set both `initialViewState` and `viewState` on deck (the documented stall).

### 4b. `mapUIStore` — interaction-rate UI (DOES re-render, but only leaf widgets)

```ts
interface MapUIStore {
  layerVisibility: Record<LayerId, boolean>;     // gyms/stops/spawns/stations/geofences/routes/s2/calc
  filters: { lastSeenRange: [number, number]; categories: Set<MarkerCategory>; };
  drawMode: 'none' | 'drawPolygon' | 'drawRectangle' | 'modify' | 'translate';
  draftFeatures: FeatureCollection;              // in-progress edits, pre-save
  selection: { kind: 'marker' | 'geofence' | 'route' | null; id: string | null };
  activeJob: { id: string; kind: 'cluster'|'route'|'bootstrap'; status: JobStatus } | null; // NOT persisted
  hoverInfo: { x: number; y: number; object: unknown } | null;
  // actions: toggleLayer, setFilterRange, setDrawMode, commitDraft, select, setActiveJob, ...
}
```

### 4c. `mapSettingsStore` — long-lived prefs, **persisted** (`persist` middleware, localStorage)

```ts
interface MapSettingsStore {
  tileServerId: string;          // which raster style.json source is active
  defaultLayerVisibility: Record<LayerId, boolean>;
  areaThresholds: { gym: number; pokestop: number; spawnpoint: number };
  markerRadius: number;
  // persist: partialize to EXCLUDE anything transient. viewState/filters/activeJob/draftFeatures are NOT persisted.
}
```

### 4d. Server data → **react-query via `dataProvider`, NOT zustand**

Markers, geofences, routes, S2 tokens, job results live in the **ra-core / react-query cache** (`useDataProvider`, `useGetList`, or a raw `dataProvider.getList` for the custom geo endpoints). Keeping them out of zustand avoids a second cache, preserves ra-core's invalidation, and keeps zustand focused on UI + transient camera. The `CustomRoute` does **not** wrap in `ResourceContextProvider` (that's a form concern) — it just calls `useDataProvider()` directly, since `<Admin>` already supplies it.

### Subscription rules (S3 discipline — hard requirement)

- **Per-field primitive selectors in leaf widgets only.** `useMapUIStore(s => s.drawMode)`, `useMapViewStore(s => s.settledViewState.zoom)`. Never select an object slice and prop-drill it.
- **No prop-drilling store slices** into children — each leaf subscribes to exactly the primitive(s) it renders. Render isolation comes from the **component boundary**, not selector cleverness.
- **No `useShallow` over a wide slice** + drill — that pattern benchmarked *slower* than naive whole-object subscription. Avoid the parent-hoisted multi-field selector (the worst pattern).
- **Derive-once for counts/labels:** computed values (visible-marker count, selected-feature label) are derived in a single small selector/`useMemo` at the consuming widget, not recomputed in N children.
- **Live camera is transient:** subscribe to `liveViewState` **only** via `subscribeWithSelector` in an effect (for deck/imperative reads), never via a `useStore` hook in render.

---

## 5. Component Breakdown (small, isolated units)

```
<MapRoute>                  CustomRoute "/map", full-bleed (h-screen w-screen). Mounts stores' providers if any, lays out canvas + overlay panels. Subscribes: nothing (pure shell).
├─ <DeckCanvas>             MapLibre root + MapboxOverlay(deck) + all layers. Owns onViewStateChange→setLive, registers throttled-settle subscription, builds memoized layer list. Subscribes (transient): liveViewState ref; (render): layerVisibility, filters, drawMode, draftFeatures, calc results from react-query.
├─ <BaseMapTiles>           Logic-only: derives MapLibre mapStyle from settings.tileServerId. Subscribes: tileServerId.
├─ <LayerDrawer>            Right/left panel: per-layer on/off + opacity. Subscribes: layerVisibility (one toggle widget per LayerId, each selecting its own bool).
├─ <DrawToolbar>            Floating tool buttons (polygon/rect/modify/translate/save/cancel). Subscribes: drawMode; calls setDrawMode/commitDraft.
├─ <FilterPanel>            last-seen time slider + category checkboxes → DataFilterExtension range. Subscribes: filters.lastSeenRange, filters.categories.
├─ <CalcPanel>              Pick op (cluster/route/bootstrap) + params, launch job, show progress, render result toggle. Subscribes: activeJob; uses react-query mutation+poll.
├─ <SelectionPopup>         Anchored card for clicked marker/geofence/route (edit/delete/inspect). Subscribes: selection, hoverInfo.
├─ <CoordinateReadout>      Tiny lng/lat/zoom HUD. Subscribes: settledViewState (primitives only).
└─ <ImportExportMenu>       (Phase 3) drop GeoJSON onto map → preview as draftFeatures; export visible geometry. Subscribes: selection, draftFeatures.
```
Each overlay panel is its own file (CLAUDE.md split rule), subscribes to only its primitives, and never receives a store slice as a prop.

---

## 6. Data Flow (real server contracts)

> **Coordinate-order & units gotchas, up front (ground-truthed against the Rust):** deck.gl / MapLibre `viewState` and `Viewport.getBounds()` are **`[lng, lat]`**; GeoJSON is also `[lng, lat]`. **Koji is `[lat, lon]` internally** (`SingleVec`), and the geofence/route GeoJSON read endpoints already auto-convert to `[lng, lat]` for you — **but the `/golbat-data` marker endpoint returns raw `[lat, lon]` arrays** (`{ points: [[lat, lon], …] }`). So markers must be **transposed** to `[lng, lat]` at the boundary; geofence/route GeoJSON must not. A single `fromKojiLatLon()` adapter pins this in one place. S2 returns **cell-id strings**; pass straight to `S2Layer` (no decode). `lastSeen` is **seconds** — `DataFilterExtension` math uses seconds, not `Date.now()` ms.

**Markers by viewport bounds.** Two real shapes exist: **`GET /api/v2/{area}/golbat-data?category={gym|pokestop|spawnpoint|station|fort}&lastSeen={secs}`** (area-scoped) and **`POST /api/v2/golbat-data/{category}`** with an `AreaReq` body that takes either a GeoJSON `area` **or** a flat `bbox` (camelCase wire). For the map we use the **POST + bbox** form on `settledBounds` change (throttled). Response is `{ points: [[lat, lon], …] }` (each `GenericData { i, p:[lat,lon] }`); spawnpoint adds a `tth` filter (`All|Known|Unknown`); `fort` is the aggregate. On receipt: transpose + pack into a **`Float32Array` `[lng,lat,…]`** fed to `ScatterplotLayer` via binary accessors. Re-fetch only when bounds move a tile; cache per-tile. GPU `DataFilterExtension` does last-seen/category narrowing **client-side** — no refetch on slider drag.

**Geofences & routes.** GeoJSON from **`GET /api/v2/geofences?format=featurecollection`** (and `/routes`), via `dataProvider.getList`/`useGetList` against the `geofence`/`route` resources. Feature `properties` carry `{ name, mode, parent, dragonite_area_id, properties[], projects[] }` (routes: `geofence_id`, `description`, geometry `MultiPoint`). Fed to `GeoJsonLayer`. Editing writes the modified `FeatureCollection` back via `dataProvider.update` — single round-trip, ra-core invalidation refreshes the layer.

**S2 cells.** **`POST /api/v2/s2/{level}`** with a `BoundsArg` (`min_lat/min_lon/max_lat/max_lon`, optional `ids` allow-list, `last_seen`/`tth`) → array of `{ id: "<cellId>", … }`. Also `/s2/circle-coverage`, `/s2/cell-coverage`, `/s2/polygons`. Cell-id strings go straight to `S2Layer` (`getS2Token: d => d.id`). Toggled + level-selected from `<LayerDrawer>`/`<FilterPanel>`.

**Calc job launch → progress → render.** `<CalcPanel>` POSTs to **`POST /api/v2/jobs`** with `CalcJobRequest { mode: "cluster"|"route"|"reroute"|"bootstrap"|"routeStats", category, …mode-specific groups (clustering/routing/bootstrap/output/dataFilter), area|dataPoints|instance }` (camelCase). It returns **`202 { job_id }`** + a `Location` header (not a held-open 504 — the spec ambiguity is resolved). Progress comes via the **realtime `jobs/{id}` topic** (events `{ status, progress, phase }`) — preferred over polling; the long-poll **`GET /api/v2/jobs/{id}?wait={secs}`** (clamped ≤290s) is the fallback and the way to fetch the terminal **`{ status, data: FeatureCollection, stats }`**. Result geometry toggles into `ScatterplotLayer`/`PathLayer` via `mapUIStore.activeJob` + a `showCalcResult` flag. `status ∈ queued|running|succeeded|failed|canceled`.

**Realtime.** One **websocket at `GET /internal/realtime`** (auth = session cookie, or `?token=<secret>`), opened once at `<MapRoute>` mount. Client frames `{ op: "subscribe"|"unsubscribe"|"ping", topic, event? }`; subscribe to `jobs/{id}` (calc progress) and `resource/geofence`/`resource/route` (mutation deltas). Server events `{ type: "status"|"progress"|"updated"|"created"|"deleted", payload }`; mutation payloads patch the react-query cache by id, and `updateTriggers` re-renders only the changed accessor. WS lifecycle tied to route mount/unmount; the socket is not stored in zustand.

---

## 7. Parity Mapping (epics 10–14)

| Epic / story | Covered by | Phase | Deferred? |
|---|---|---|---|
| **10** Draw geofence (polygon/rect/circle) | `EditableGeoJsonLayer` draw modes + `<DrawToolbar>` | 2 | — |
| **10** Edit/modify/translate existing geofence | `EditableGeoJsonLayer` modify/translate; save via `dataProvider.update` | 2 | — |
| **10** Snap / vertex precision tools | basic vertex edit yes; advanced snapping | 2 | snapping → Phase 3 |
| **11** Marker layers (gym/stop/spawn/station) | `ScatterplotLayer` + viewport fetch (`/api/v2/golbat-data`) | 1 | — |
| **11** S2 cell overlay (multi-level) | `S2Layer` ← `/s2` | 1 | — |
| **11** Layer toggle + opacity | `<LayerDrawer>` ← `layerVisibility` | 1 | — |
| **11** Marker filtering (time/category) | `DataFilterExtension` + `<FilterPanel>` | 1/2 | — |
| **11** Dense-area heat | `HexagonLayer` | 2 | optional |
| **12** Launch clustering/routing/bootstrap | `<CalcPanel>` → `POST /api/v2/jobs` (`mode: cluster\|route\|bootstrap`) | 3 | — |
| **12** Job progress / poll / cancel | realtime `jobs/{id}` topic (`{status,progress,phase}`) + `GET /jobs/{id}?wait=N` fallback | 3 | cancel → Phase 3 tail |
| **12** Render calc results on map | `ScatterplotLayer`/`PathLayer` result layers | 3 | — |
| **12** Calc param presets / area thresholds | `mapSettingsStore.areaThresholds` | 3 | nice-to-haves deferred |
| **13** Import GeoJSON onto map | `<ImportExportMenu>` → `draftFeatures` preview | 3 | — |
| **13** Export visible/selected geometry | `<ImportExportMenu>` serialize layer GeoJSON | 3 | — |
| **14** Pan/zoom/fit-bounds nav | MapLibre controller + uncontrolled view | 1 | — |
| **14** Click popups / inspect | `<SelectionPopup>` ← `selection`/`hoverInfo` | 1 | — |
| **14** Tile-server switch | `mapStyle` swap ← `mapSettingsStore.tileServerId` | 1 | — |
| **14** Persisted map settings | `persist` middleware | 1 | — |
| **14** Realtime marker updates | websocket → react-query patch | 2 | can land Phase 1 tail if WS contract ready |

---

## 8. Phasing (lazy / incremental)

**Phase 1 — MVP read-only map.** MapLibre+deck overlay shell; `<DeckCanvas>` uncontrolled camera + transient store; raster base + **tile-server switch**; viewport-bounds marker fetch (`/api/v2/golbat-data`) → `ScatterplotLayer`; geofences/routes via `GeoJsonLayer`; S2 via `S2Layer`; `<LayerDrawer>`, `<CoordinateReadout>`, `<SelectionPopup>`; persisted `mapSettingsStore`. **No editing, no calc.**

**Phase 2 — drawing/editing + filters + realtime.** `EditableGeoJsonLayer` + `<DrawToolbar>`; save edits via `dataProvider.update`; `<FilterPanel>` + `DataFilterExtension`; optional `HexagonLayer`; websocket marker deltas.

**Phase 3 — calc UI + jobs + import/export.** `<CalcPanel>` launch→poll→render against `/api/v2/calc` + `/jobs/{id}`; result layers; `<ImportExportMenu>` GeoJSON in/out; advanced snapping + job cancel as the tail.

Each phase ships behind the single `/map` route; later phases add panels and layers without touching the Phase-1 canvas core.

---

## 9. Risks & Open Questions

- **[RESOLVED] `@deck.gl-community/editable-layers` fit** — confirmed v9.3.7 (active, 2026-06), pins `@deck.gl/core@~9.3.0`, React 19 OK, pure ESM (Vite-clean). Has all needed modes; merge/union done with turf. Re-confirm the exact tag at install but no fallback expected.
- **[RESOLVED] `golbat-data` contract** — confirmed: `POST /golbat-data/{category}` with `AreaReq{area|bbox}` (camelCase) → `{points:[[lat,lon],…]}`. Markers are `[lat,lon]` and MUST be transposed for deck; the `fromKojiLatLon` adapter pins it.
- **[RESOLVED] Job endpoint semantics** — confirmed `POST /jobs → 202 {job_id}`, `GET /jobs/{id}?wait=N` long-poll ≤290s, result `{status,data,stats}`. Realtime `jobs/{id}` topic gives `{status,progress,phase}`. No 504 sync-bridge on this path.
- **[OPEN] Binary marker pipeline off-by-one** — packed-array stride/offset bugs render as ghost points; build the `Float32Array` path against a tiny fixture first, then scale.
- **[OPEN] deck.gl React-19 "Activity" view-reset bug** ([visgl/deck.gl#9983](https://github.com/visgl/deck.gl/issues/9983)) — hidden→shown remounts reset `initialViewState`. Our uncontrolled-camera-+-external-store pattern already side-steps it (manage viewState outside React mounting); verify on first integration.
- **[OPEN] react-map-gl version** — `^8.1.x` is current; don't future-target 9.x.
- **[OPEN] Tile-server style.json shape** — need the actual raster source template URLs (from the `tile_server` resource `url`) to build the `mapStyle` factory; assumed `{z}/{x}/{y}.png` XYZ. Confirm one real tile server.

---

## 10. Assumptions (owner review checkpoint)

Each line is a judgement call made on the owner's behalf — accept/reject individually.

1. **Base map = MapLibre GL JS as root + deck.gl `MapboxOverlay` (overlay, non-interleaved)** — *not* pure deck `TileLayer`, *not* Leaflet. (Adds `maplibre-gl` + `react-map-gl` deps.)
2. **Editing = `@deck.gl-community/editable-layers` (EditableGeoJsonLayer)**, deck-native — **leaflet/geoman is NOT used on the map page** (kept only for resource forms).
3. **Leaflet/react-leaflet/geoman are dropped from the map page entirely** (no `deck.gl-leaflet` bridge).
4. **Raster-only base map** (XYZ `{z}/{x}/{y}.png`); no vector tiles, ever, for this page. Tile-server switch = `mapStyle` swap.
5. **Full-bleed dedicated route** (`h-screen`/`w-screen` CustomRoute), not embedded inside the admin list/edit layout chrome.
6. **Server data (markers/geofences/routes/S2/jobs) stays in react-query via `dataProvider`** — *not* mirrored into zustand. Only UI + transient camera + persisted settings live in zustand.
7. **Camera is uncontrolled in deck/MapLibre**; live `viewState` is a **transient ref write** (no React render at 60fps); a **throttled `settledViewState`** drives the few UI widgets that react. We do **not** set controlled `viewState` + `initialViewState` together.
8. **Three zustand stores** split by update-frequency (view / UI / settings), not one monolith.
9. **`persist` middleware persists settings only** (tile server, default layer visibility, thresholds, radius) — **not** viewState, filters, activeJob, or draftFeatures.
10. **S2 tokens passed straight to `S2Layer`** (no client-side decode).
11. **Markers use binary `Float32Array` packed positions** + `ScatterplotLayer` (not `IconLayer`); custom marker icons deferred. Time/category narrowing is **GPU client-side** (`DataFilterExtension`), not refetch.
12. **Calc/jobs are async** `POST /api/v2/jobs` → `202 {job_id}`; **progress via realtime `jobs/{id}` topic**, terminal result via `GET /jobs/{id}?wait=N` (≤290s). Sync-bridge/504 not used.
13. **A single coordinate adapter** (`toKojiLatLon`/`fromKojiLatLon`) mediates all deck↔Koji boundary crossings; lat,lon vs lng,lat is pinned in one place.
14. **zustand@5 + `subscribeWithSelector`** is the subscription primitive; **no `useShallow`-over-wide-slice**, per S3 discipline.
15. **Phasing is read-map first (P1), editing+realtime (P2), calc+import/export (P3)** — editing and calc are deliberately *not* in the MVP.