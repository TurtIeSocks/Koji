# Koji v2 deck.gl Map — Phase 2 (Editing + Filters + Realtime) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add geometry drawing/editing (geofences + routes), server-backed marker filtering, and live realtime marker/geofence deltas to the existing read-only `/map` page in `apps/web`.

**Architecture:** Drawing/editing uses deck.gl's `@deck.gl-community/editable-layers` `EditableGeoJsonLayer` rendered on the same canvas (no leaflet/geoman on the map). The in-progress edit `FeatureCollection`, draw mode, and selected indexes live in the existing `mapUIStore` (zustand). Saving writes back via the ra-core `dataProvider` on the `geofence`/`route` resources. Filtering is **server-side re-query** (the `/golbat-data` payload carries no per-point attributes, so GPU `DataFilterExtension` is not viable). Realtime reuses the app's existing websocket client (`src/components/realtime`) to patch the TanStack Query cache.

**Tech Stack:** React 19, TypeScript 6, Vite 8, deck.gl 9.3, `@deck.gl-community/editable-layers` 9.3.x, zustand 5, TanStack Query 5, turf 7, ra-core dataProvider.

## Global Constraints

- **Builds on Phase 1** (`docs/superpowers/plans/2026-06-26-koji-v2-map-phase1.md`, commits `4031cc80..fad0bc15`). Existing, DO NOT recreate: `src/map/{map-route,deck-canvas}.tsx`, `src/map/stores/{types,map-view-store,map-ui-store,map-settings-store}.ts`, `src/map/lib/{coords,map-style,layers}.ts`, `src/map/data/{use-markers,use-geo-features,use-s2-cells}.ts`, `src/map/panels/{layer-drawer,coordinate-readout,tile-server-select,selection-popup}.tsx`.
- **New dependency:** `@deck.gl-community/editable-layers@^9.3.7` (pins `@deck.gl/core@~9.3.0`; React 19 OK; pure ESM). Do NOT bump `@deck.gl/*` or `luma.gl`.
- **Zustand discipline (hard requirement):** narrowest primitive selector in the consuming leaf component (S3); no prop-drilled slices; no wide `useShallow`; derive-once. New `drawMode`/`draftFeatures`/`filters` fields go in the existing `mapUIStore`; panels subscribe per-field.
- **Editing data shape:** `EditableGeoJsonLayer` holds `data: FeatureCollection`, `mode`, `selectedFeatureIndexes`, and `onEdit({updatedData, editType})` → write `updatedData` to `mapUIStore.draftFeatures`. Geofence geometry is Polygon/MultiPolygon (`[lng,lat]`); route geometry is MultiPoint. **Merge/union is NOT a built-in mode** → use `@turf/union` + `@turf/difference` (already deps).
- **Coordinates:** edits are GeoJSON `[lng,lat]` end-to-end — no transpose (only `/golbat-data` markers are `[lat,lon]`, handled in Phase 1's `coords.ts`).
- **Filtering is server re-query, NOT GPU:** `/golbat-data` returns only `{points:[[lat,lon]]}` (no per-point `last_seen`/category), so filter changes re-issue the marker query with new params (`lastSeen`, and `tth` for spawnpoints). Debounce the slider.
- **Realtime:** reuse `src/components/realtime` (websocket transport already built); subscribe to `resource/geofence`, `resource/route`; on a mutation event, invalidate/patch the relevant TanStack Query keys. Do NOT store the socket in zustand.
- **Test commands:** unit `bun run test`; browser `bun run test:browser` (FOREGROUND); types `bun run typecheck`; build `bun run build`. Unit = `*.test.ts(x)`, browser = `*.browser.test.tsx`.
- **No v1 copy.** Files stay focused (one unit per file).

---

## File Structure

```
apps/web/src/map/
  stores/
    map-ui-store.ts        # MODIFY: + drawMode, draftFeatures, selectedFeatureIndexes, filters + actions (Task 2)
  lib/
    edit-modes.ts          # NEW: DrawMode union -> editable-layers mode class map (Task 1)
    edit-serialize.ts      # NEW: draftFeatures -> geofence/route write payloads (Task 4)
    merge-polygons.ts      # NEW: turf union of selected polygon features (Task 5)
    layers.ts              # MODIFY: append EditableGeoJsonLayer when a draw mode is active (Task 3)
  data/
    use-markers.ts         # MODIFY: thread tth param; (lastSeen already a param) (Task 6)
    use-map-realtime.ts    # NEW: subscribe geofence/route topics -> invalidate query cache (Task 7)
  panels/
    draw-toolbar.tsx       # NEW: draw/modify/translate/merge/save/cancel (Tasks 3,5)
    filter-panel.tsx       # NEW: last-seen slider + spawnpoint TTH (Task 6)
  deck-canvas.tsx          # MODIFY: pass draft + drawMode to buildLayers; onEdit -> store; mount realtime (Tasks 3,6,7)
  map-route.tsx            # MODIFY: mount <DrawToolbar/> + <FilterPanel/> (Tasks 3,6)
```

---

### Task 1: Dependency + draw-mode map

**Files:**
- Modify: `apps/web/package.json` (+ `bun.lock`)
- Create: `apps/web/src/map/lib/edit-modes.ts`
- Create: `apps/web/src/map/lib/edit-modes.test.ts`

**Interfaces:**
- Produces: `type DrawMode = "none" | "drawPolygon" | "drawRectangle" | "modify" | "translate"`; `editModeFor(mode: DrawMode): unknown` returning the editable-layers mode class (or `ViewMode` for `"none"`).

- [ ] **Step 1: Add the dependency**

```bash
cd apps/web && bun add @deck.gl-community/editable-layers@^9.3.7
```
Commit `package.json` + `bun.lock` together.

- [ ] **Step 2: Write the failing test**

`apps/web/src/map/lib/edit-modes.test.ts`:

```ts
import { expect, test } from "vitest";
import { editModeFor } from "@/map/lib/edit-modes";
import { DrawPolygonMode, ModifyMode, ViewMode } from "@deck.gl-community/editable-layers";

test("editModeFor maps each DrawMode to its editable-layers class", () => {
  expect(editModeFor("none")).toBe(ViewMode);
  expect(editModeFor("drawPolygon")).toBe(DrawPolygonMode);
  expect(editModeFor("modify")).toBe(ModifyMode);
});
```

- [ ] **Step 3: Run test to verify it fails**

Run: `bun run test -- edit-modes`
Expected: FAIL — module not found.

- [ ] **Step 4: Implement `edit-modes.ts`**

```ts
import {
  ViewMode, DrawPolygonMode, DrawRectangleMode, ModifyMode, TranslateMode,
} from "@deck.gl-community/editable-layers";

export type DrawMode = "none" | "drawPolygon" | "drawRectangle" | "modify" | "translate";

const MAP = {
  none: ViewMode,
  drawPolygon: DrawPolygonMode,
  drawRectangle: DrawRectangleMode,
  modify: ModifyMode,
  translate: TranslateMode,
} as const;

export function editModeFor(mode: DrawMode): unknown {
  return MAP[mode];
}
```

- [ ] **Step 5: Run test to verify it passes**

Run: `bun run test -- edit-modes`
Expected: PASS (1 test).

- [ ] **Step 6: Commit**

```bash
git add apps/web/package.json apps/web/bun.lock apps/web/src/map/lib/edit-modes.ts apps/web/src/map/lib/edit-modes.test.ts
git commit -m "feat(web/map): editable-layers dep + draw-mode class map"
```

---

### Task 2: Editing + filter state in `mapUIStore`

**Files:**
- Modify: `apps/web/src/map/stores/map-ui-store.ts`
- Modify: `apps/web/src/map/stores/map-ui-store.test.ts`

**Interfaces:**
- Consumes: `DrawMode` (Task 1).
- Produces (added to `MapUIState`):
  ```ts
  drawMode: DrawMode;
  draftFeatures: GeoJSON.FeatureCollection;     // editable-layers data buffer
  selectedFeatureIndexes: number[];
  filters: { lastSeen: number; tth: "All" | "Known" | "Unknown" };
  setDrawMode: (m: DrawMode) => void;            // switching to "none" keeps the draft
  setDraftFeatures: (fc: GeoJSON.FeatureCollection) => void;
  setSelectedFeatureIndexes: (ix: number[]) => void;
  clearDraft: () => void;                         // empties draft + indexes, mode -> none
  setLastSeen: (secs: number) => void;
  setTth: (t: "All" | "Known" | "Unknown") => void;
  ```

- [ ] **Step 1: Write the failing tests**

Add to `apps/web/src/map/stores/map-ui-store.test.ts`:

```ts
test("setDraftFeatures + clearDraft manage the edit buffer", () => {
  const fc: GeoJSON.FeatureCollection = {
    type: "FeatureCollection",
    features: [{ type: "Feature", properties: {}, geometry: { type: "Point", coordinates: [1, 2] } }],
  };
  useMapUIStore.getState().setDrawMode("drawPolygon");
  useMapUIStore.getState().setDraftFeatures(fc);
  useMapUIStore.getState().setSelectedFeatureIndexes([0]);
  expect(useMapUIStore.getState().draftFeatures.features).toHaveLength(1);
  useMapUIStore.getState().clearDraft();
  expect(useMapUIStore.getState().draftFeatures.features).toHaveLength(0);
  expect(useMapUIStore.getState().selectedFeatureIndexes).toEqual([]);
  expect(useMapUIStore.getState().drawMode).toBe("none");
});

test("filter actions update lastSeen and tth independently", () => {
  useMapUIStore.getState().setLastSeen(3600);
  useMapUIStore.getState().setTth("Known");
  expect(useMapUIStore.getState().filters).toEqual({ lastSeen: 3600, tth: "Known" });
});
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `bun run test -- map-ui-store`
Expected: FAIL — new actions undefined.

- [ ] **Step 3: Extend the store**

In `apps/web/src/map/stores/map-ui-store.ts`, import `DrawMode`, add the fields to `MapUIState`, seed initial state (`drawMode: "none"`, `draftFeatures: { type: "FeatureCollection", features: [] }`, `selectedFeatureIndexes: []`, `filters: { lastSeen: 0, tth: "All" }`), and implement the actions:

```ts
import type { DrawMode } from "@/map/lib/edit-modes";
// ... inside create<MapUIState>()((set) => ({ ...existing,
  drawMode: "none",
  draftFeatures: { type: "FeatureCollection", features: [] },
  selectedFeatureIndexes: [],
  filters: { lastSeen: 0, tth: "All" },
  setDrawMode: (m) => set({ drawMode: m }),
  setDraftFeatures: (fc) => set({ draftFeatures: fc }),
  setSelectedFeatureIndexes: (ix) => set({ selectedFeatureIndexes: ix }),
  clearDraft: () =>
    set({ draftFeatures: { type: "FeatureCollection", features: [] }, selectedFeatureIndexes: [], drawMode: "none" }),
  setLastSeen: (secs) => set((s) => ({ filters: { ...s.filters, lastSeen: secs } })),
  setTth: (t) => set((s) => ({ filters: { ...s.filters, tth: t } })),
// }))
```
Add the matching field + action signatures to the `MapUIState` interface.

- [ ] **Step 4: Run tests to verify they pass**

Run: `bun run test -- map-ui-store`
Expected: PASS (all map-ui-store tests).

- [ ] **Step 5: Commit**

```bash
git add apps/web/src/map/stores/map-ui-store.ts apps/web/src/map/stores/map-ui-store.test.ts
git commit -m "feat(web/map): editing + filter state in map UI store"
```

---

### Task 3: Editable layer + DrawToolbar

**Files:**
- Modify: `apps/web/src/map/lib/layers.ts`
- Modify: `apps/web/src/map/lib/layers.test.ts`
- Create: `apps/web/src/map/panels/draw-toolbar.tsx`
- Create: `apps/web/src/map/panels/draw-toolbar.browser.test.tsx`
- Modify: `apps/web/src/map/deck-canvas.tsx`
- Modify: `apps/web/src/map/map-route.tsx`

**Interfaces:**
- Consumes: `DrawMode`/`editModeFor` (Task 1), draft state (Task 2).
- Produces: `buildLayers` gains `draft?: { mode: DrawMode; features: GeoJSON.FeatureCollection; selectedIndexes: number[]; onEdit: (e: { updatedData: GeoJSON.FeatureCollection }) => void }`. When `draft && draft.mode !== "none"`, append an `EditableGeoJsonLayer` (id `"edit"`). `<DrawToolbar>` renders mode buttons + Save/Cancel.

- [ ] **Step 1: Write the failing layer test**

Add to `apps/web/src/map/lib/layers.test.ts`:

```ts
import { vi as _vi } from "vitest";
test("buildLayers appends an editable layer only when a draw mode is active", () => {
  const draftOff = buildLayers({
    visibility: vis(), markerSets: [], geofences: EMPTY, routes: EMPTY, s2Cells: [],
    markerRadius: 30, onClick: _vi.fn(),
    draft: { mode: "none", features: EMPTY, selectedIndexes: [], onEdit: _vi.fn() },
  });
  expect(draftOff.find((l) => l.id === "edit")).toBeUndefined();

  const draftOn = buildLayers({
    visibility: vis(), markerSets: [], geofences: EMPTY, routes: EMPTY, s2Cells: [],
    markerRadius: 30, onClick: _vi.fn(),
    draft: { mode: "drawPolygon", features: EMPTY, selectedIndexes: [], onEdit: _vi.fn() },
  });
  expect(draftOn.find((l) => l.id === "edit")).toBeDefined();
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `bun run test -- layers`
Expected: FAIL — `draft` not handled; no `"edit"` layer.

- [ ] **Step 3: Extend `buildLayers`**

In `apps/web/src/map/lib/layers.ts`: import `EditableGeoJsonLayer` from `@deck.gl-community/editable-layers` and `editModeFor` from `@/map/lib/edit-modes`; add the optional `draft` to `BuildLayersInput`; after the existing layers, conditionally append:

```ts
import { EditableGeoJsonLayer } from "@deck.gl-community/editable-layers";
import { editModeFor, type DrawMode } from "@/map/lib/edit-modes";
// BuildLayersInput += 
//   draft?: { mode: DrawMode; features: GeoJSON.FeatureCollection; selectedIndexes: number[];
//             onEdit: (e: { updatedData: GeoJSON.FeatureCollection }) => void };
// at the end of the returned array:
const editLayer =
  input.draft && input.draft.mode !== "none"
    ? [
        new EditableGeoJsonLayer({
          id: "edit",
          data: input.draft.features,
          mode: editModeFor(input.draft.mode),
          selectedFeatureIndexes: input.draft.selectedIndexes,
          onEdit: input.draft.onEdit,
          getFillColor: [0, 150, 255, 60],
          getLineColor: [0, 150, 255, 220],
        }),
      ]
    : [];
return [...baseLayers, ...editLayer];
```
(Assign the existing returned array to `const baseLayers` and spread.)

- [ ] **Step 4: Run test to verify it passes**

Run: `bun run test -- layers`
Expected: PASS.

- [ ] **Step 5: Write the DrawToolbar + its failing browser test**

`apps/web/src/map/panels/draw-toolbar.browser.test.tsx`:

```tsx
import { render } from "vitest-browser-react";
import { beforeEach, expect, test } from "vitest";
import { DrawToolbar } from "@/map/panels/draw-toolbar";
import { useMapUIStore } from "@/map/stores/map-ui-store";

beforeEach(() => useMapUIStore.setState(useMapUIStore.getInitialState()));

test("clicking Polygon sets the draw mode; Cancel clears the draft", async () => {
  const screen = render(<DrawToolbar />);
  await screen.getByRole("button", { name: /polygon/i }).click();
  expect(useMapUIStore.getState().drawMode).toBe("drawPolygon");
  await screen.getByRole("button", { name: /cancel/i }).click();
  expect(useMapUIStore.getState().drawMode).toBe("none");
});
```

`apps/web/src/map/panels/draw-toolbar.tsx`:

```tsx
import { Button } from "@/components/ui/button";
import { useMapUIStore } from "@/map/stores/map-ui-store";
import type { DrawMode } from "@/map/lib/edit-modes";

const MODES: { mode: DrawMode; label: string }[] = [
  { mode: "drawPolygon", label: "Polygon" },
  { mode: "drawRectangle", label: "Rectangle" },
  { mode: "modify", label: "Modify" },
  { mode: "translate", label: "Move" },
];

/** Leaf: subscribes to drawMode + the edit actions only (S3). Save is wired by Task 4. */
export function DrawToolbar() {
  const drawMode = useMapUIStore((s) => s.drawMode);
  const setDrawMode = useMapUIStore((s) => s.setDrawMode);
  const clearDraft = useMapUIStore((s) => s.clearDraft);
  return (
    <div className="absolute bottom-4 left-1/2 z-10 flex -translate-x-1/2 gap-1 rounded-lg border bg-background/90 p-1 shadow-md backdrop-blur">
      {MODES.map((m) => (
        <Button
          key={m.mode}
          size="sm"
          variant={drawMode === m.mode ? "default" : "ghost"}
          onClick={() => setDrawMode(m.mode)}
        >
          {m.label}
        </Button>
      ))}
      <Button size="sm" variant="ghost" onClick={clearDraft}>Cancel</Button>
    </div>
  );
}
```

- [ ] **Step 6: Wire the canvas + route**

In `apps/web/src/map/deck-canvas.tsx`: subscribe `drawMode`, `draftFeatures`, `selectedFeatureIndexes`, and the `setDraftFeatures`/`setSelectedFeatureIndexes` actions (per-field), and pass a `draft` object into `buildLayers` with `onEdit: (e) => setDraftFeatures(e.updatedData)`. Add the draft deps to the `useMemo` dep array. In `apps/web/src/map/map-route.tsx`, mount `<DrawToolbar />` alongside the other panels.

- [ ] **Step 7: Run tests (unit + browser) to verify they pass**

Run: `bun run test -- layers` and `bun run test:browser -- draw-toolbar` (FOREGROUND).
Expected: PASS.

- [ ] **Step 8: Commit**

```bash
git add apps/web/src/map/lib/layers.ts apps/web/src/map/lib/layers.test.ts \
  apps/web/src/map/panels/draw-toolbar.tsx apps/web/src/map/panels/draw-toolbar.browser.test.tsx \
  apps/web/src/map/deck-canvas.tsx apps/web/src/map/map-route.tsx
git commit -m "feat(web/map): editable geojson layer + draw toolbar"
```

---

### Task 4: Serialize + save edits to the geofence/route resource

**Files:**
- Create: `apps/web/src/map/lib/edit-serialize.ts`
- Create: `apps/web/src/map/lib/edit-serialize.test.ts`
- Modify: `apps/web/src/map/panels/draw-toolbar.tsx`
- Modify: `apps/web/src/map/panels/draw-toolbar.browser.test.tsx`

**Interfaces:**
- Produces: `firstGeometry(fc: GeoJSON.FeatureCollection): GeoJSON.Geometry | null` (the drawn shape to persist). `<DrawToolbar>` gains a **Save** button that, when `draftFeatures` has a feature, calls `dataProvider.create("geofence", { data: { geometry, mode: "Unset", name: <generated> } })` via `useDataProvider`, then `clearDraft()` + a success notification.

- [ ] **Step 1: Write the failing serialize test**

`apps/web/src/map/lib/edit-serialize.test.ts`:

```ts
import { expect, test } from "vitest";
import { firstGeometry } from "@/map/lib/edit-serialize";

test("firstGeometry returns the geometry of the first drawn feature, or null", () => {
  const geom = { type: "Polygon", coordinates: [[[0, 0], [1, 0], [1, 1], [0, 0]]] } as const;
  const fc: GeoJSON.FeatureCollection = {
    type: "FeatureCollection",
    features: [{ type: "Feature", properties: {}, geometry: geom }],
  };
  expect(firstGeometry(fc)).toEqual(geom);
  expect(firstGeometry({ type: "FeatureCollection", features: [] })).toBeNull();
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `bun run test -- edit-serialize`
Expected: FAIL — module not found.

- [ ] **Step 3: Implement `edit-serialize.ts`**

```ts
export function firstGeometry(fc: GeoJSON.FeatureCollection): GeoJSON.Geometry | null {
  return fc.features[0]?.geometry ?? null;
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `bun run test -- edit-serialize`
Expected: PASS.

- [ ] **Step 5: Add Save to DrawToolbar**

In `draw-toolbar.tsx`, add a Save button: read `draftFeatures` (selector), `useDataProvider()`, `useNotify()`. On click, `const geometry = firstGeometry(draftFeatures); if (!geometry) return;` then `await dataProvider.create("geofence", { data: { name: \`map-${Date.now()}\`, mode: "Unset", geometry } })`, then `clearDraft()` + `notify("Geofence saved", { type: "info" })`. Update the browser test to mock `useDataProvider` (a `create` spy) and `useNotify`, draw a feature into the store, click Save, and assert `create` was called with the geometry. (Follow the Phase-1/slice-3 shadmin test gotchas: spy `useNotify`, don't assert toast text.)

- [ ] **Step 6: Run tests to verify they pass**

Run: `bun run test -- edit-serialize` and `bun run test:browser -- draw-toolbar` (FOREGROUND).
Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add apps/web/src/map/lib/edit-serialize.ts apps/web/src/map/lib/edit-serialize.test.ts \
  apps/web/src/map/panels/draw-toolbar.tsx apps/web/src/map/panels/draw-toolbar.browser.test.tsx
git commit -m "feat(web/map): save drawn geometry to the geofence resource"
```

---

### Task 5: Merge selected polygons (turf)

**Files:**
- Create: `apps/web/src/map/lib/merge-polygons.ts`
- Create: `apps/web/src/map/lib/merge-polygons.test.ts`
- Modify: `apps/web/src/map/panels/draw-toolbar.tsx`

**Interfaces:**
- Consumes: `@turf/union`.
- Produces: `mergeSelected(fc: GeoJSON.FeatureCollection, indexes: number[]): GeoJSON.FeatureCollection` — unions the selected polygon features into one, leaving others intact. `<DrawToolbar>` gains a **Merge** button (enabled when ≥2 selected) that sets `draftFeatures` to the merged result.

- [ ] **Step 1: Write the failing test**

`apps/web/src/map/lib/merge-polygons.test.ts`:

```ts
import { expect, test } from "vitest";
import { mergeSelected } from "@/map/lib/merge-polygons";

const poly = (x: number): GeoJSON.Feature => ({
  type: "Feature", properties: {},
  geometry: { type: "Polygon", coordinates: [[[x, 0], [x + 2, 0], [x + 2, 2], [x, 2], [x, 0]]] },
});

test("mergeSelected unions two overlapping polygons into one feature", () => {
  const fc: GeoJSON.FeatureCollection = { type: "FeatureCollection", features: [poly(0), poly(1)] };
  const out = mergeSelected(fc, [0, 1]);
  expect(out.features).toHaveLength(1);
  expect(["Polygon", "MultiPolygon"]).toContain(out.features[0].geometry.type);
});

test("mergeSelected with <2 indexes returns the input unchanged", () => {
  const fc: GeoJSON.FeatureCollection = { type: "FeatureCollection", features: [poly(0)] };
  expect(mergeSelected(fc, [0])).toBe(fc);
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `bun run test -- merge-polygons`
Expected: FAIL — module not found.

- [ ] **Step 3: Implement `merge-polygons.ts`**

```ts
import { union } from "@turf/union";
import { featureCollection } from "@turf/helpers";

export function mergeSelected(
  fc: GeoJSON.FeatureCollection,
  indexes: number[],
): GeoJSON.FeatureCollection {
  if (indexes.length < 2) return fc;
  const selected = indexes.map((i) => fc.features[i]).filter(Boolean);
  const merged = union(featureCollection(selected as never));
  if (!merged) return fc;
  const rest = fc.features.filter((_, i) => !indexes.includes(i));
  return { type: "FeatureCollection", features: [...rest, merged] };
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `bun run test -- merge-polygons`
Expected: PASS. (turf 7's `union` takes a `FeatureCollection`.)

- [ ] **Step 5: Add Merge to DrawToolbar**

Add a Merge button: read `selectedFeatureIndexes` + `draftFeatures` (selectors) + `setDraftFeatures` + `setSelectedFeatureIndexes`; `disabled={selectedFeatureIndexes.length < 2}`; on click `setDraftFeatures(mergeSelected(draftFeatures, selectedFeatureIndexes)); setSelectedFeatureIndexes([]);`.

- [ ] **Step 6: Run tests + a self-check**

Run: `bun run test -- merge-polygons`
Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add apps/web/src/map/lib/merge-polygons.ts apps/web/src/map/lib/merge-polygons.test.ts apps/web/src/map/panels/draw-toolbar.tsx
git commit -m "feat(web/map): merge selected polygons via turf union"
```

---

### Task 6: Marker filtering (server re-query) + FilterPanel

**Files:**
- Modify: `apps/web/src/map/data/use-markers.ts`
- Modify: `apps/web/src/map/data/use-markers.test.ts`
- Create: `apps/web/src/map/panels/filter-panel.tsx`
- Create: `apps/web/src/map/panels/filter-panel.browser.test.tsx`
- Modify: `apps/web/src/map/deck-canvas.tsx`
- Modify: `apps/web/src/map/map-route.tsx`

**Interfaces:**
- Produces: `fetchMarkers(category, bounds, lastSeen, tth?)` gains an optional `tth: "All"|"Known"|"Unknown"` sent only for spawnpoint; `useMarkers(category, bounds, lastSeen, enabled, tth?)` keys on `tth`. `<FilterPanel>` = a last-seen slider (writes `setLastSeen`) + a spawnpoint TTH select (writes `setTth`). `<DeckCanvas>` reads `filters.lastSeen`/`filters.tth` and threads them into the marker hooks (debounce the lastSeen value before it hits the query key).

- [ ] **Step 1: Write the failing marker test**

Add to `apps/web/src/map/data/use-markers.test.ts`:

```ts
test("fetchMarkers sends tth only for spawnpoint", async () => {
  const fetchMock = vi.fn().mockResolvedValue(
    new Response(JSON.stringify({ status: "ok", data: { points: [] } }), { status: 200 }),
  );
  vi.stubGlobal("fetch", fetchMock);
  await fetchMarkers("spawnpoint", B, 100, "Known");
  expect(JSON.parse(fetchMock.mock.calls[0][1].body)).toMatchObject({ lastSeen: 100, tth: "Known" });
  fetchMock.mockClear();
  await fetchMarkers("gym", B, 100, "Known");
  expect(JSON.parse(fetchMock.mock.calls[0][1].body).tth).toBeUndefined();
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `bun run test -- use-markers`
Expected: FAIL — `tth` not sent.

- [ ] **Step 3: Thread `tth` through the marker fetch**

In `use-markers.ts`, add `tth?: "All" | "Known" | "Unknown"` to `fetchMarkers`; build the body as `{ bbox: ..., lastSeen, ...(category === "spawnpoint" && tth ? { tth } : {}) }`. Add `tth` to `useMarkers` params + query key.

- [ ] **Step 4: Run test to verify it passes**

Run: `bun run test -- use-markers`
Expected: PASS.

- [ ] **Step 5: Build FilterPanel + its failing browser test**

`apps/web/src/map/panels/filter-panel.browser.test.tsx`:

```tsx
import { render } from "vitest-browser-react";
import { beforeEach, expect, test } from "vitest";
import { FilterPanel } from "@/map/panels/filter-panel";
import { useMapUIStore } from "@/map/stores/map-ui-store";

beforeEach(() => useMapUIStore.setState(useMapUIStore.getInitialState()));

test("selecting a TTH option updates the store", async () => {
  const screen = render(<FilterPanel />);
  // shadcn Select: open then pick (combobox-by-text, not label — shadmin gotcha)
  await screen.getByRole("combobox").click();
  await screen.getByText("Known").click();
  expect(useMapUIStore.getState().filters.tth).toBe("Known");
});
```

`apps/web/src/map/panels/filter-panel.tsx`: a small panel (`absolute bottom-4 right-4 z-10`) with a last-seen `<Slider>` (writes `setLastSeen`, value `filters.lastSeen`) and a TTH `<Select>` (`All`/`Known`/`Unknown`, writes `setTth`). Each control subscribes to its own primitive (S3).

- [ ] **Step 6: Wire canvas + route**

In `deck-canvas.tsx`, subscribe `filters.lastSeen` + `filters.tth` (per-field), debounce `lastSeen` (a `useDebounce`-style `useState`+`useEffect`, 400ms) and pass it + `tth` into the four `useMarkers` calls (`tth` only matters for spawnpoint). Mount `<FilterPanel />` in `map-route.tsx`.

- [ ] **Step 7: Run tests to verify they pass**

Run: `bun run test -- use-markers` and `bun run test:browser -- filter-panel` (FOREGROUND).
Expected: PASS.

- [ ] **Step 8: Commit**

```bash
git add apps/web/src/map/data/use-markers.ts apps/web/src/map/data/use-markers.test.ts \
  apps/web/src/map/panels/filter-panel.tsx apps/web/src/map/panels/filter-panel.browser.test.tsx \
  apps/web/src/map/deck-canvas.tsx apps/web/src/map/map-route.tsx
git commit -m "feat(web/map): marker filtering by last-seen + spawnpoint TTH (server re-query)"
```

---

### Task 7: Realtime geofence/route deltas

**Files:**
- Create: `apps/web/src/map/data/use-map-realtime.ts`
- Create: `apps/web/src/map/data/use-map-realtime.test.ts`
- Modify: `apps/web/src/map/deck-canvas.tsx`

**Interfaces:**
- Consumes: the app's realtime subscribe hook (`src/components/realtime` — confirm the exported hook name, e.g. `useSubscribe`, by reading `src/components/realtime/index.ts` first) and `useQueryClient` from TanStack Query.
- Produces: `useMapRealtime()` — subscribes to topics `resource/geofence` and `resource/route`; on any event, invalidates the `["geo","geofences"]` / `["geo","routes"]` query keys so the GeoJSON layers refetch. No store writes; cleanup on unmount.

- [ ] **Step 1: Read the realtime client surface**

Run: `cat apps/web/src/components/realtime/index.ts` and `sed -n '1,60p' apps/web/src/components/realtime/hooks/use-subscribe.ts` to learn the exact subscribe hook signature + topic format. Use that signature in the code below (adjust the hook name/args to match).

- [ ] **Step 2: Write the failing test**

`apps/web/src/map/data/use-map-realtime.test.ts` (mock the realtime hook + a query client):

```ts
import { expect, test, vi } from "vitest";
import { renderHook } from "vitest-browser-react";

const invalidate = vi.fn();
vi.mock("@tanstack/react-query", () => ({ useQueryClient: () => ({ invalidateQueries: invalidate }) }));
const handlers: Record<string, (e: unknown) => void> = {};
vi.mock("@/components/realtime", () => ({
  useSubscribe: (topic: string, cb: (e: unknown) => void) => { handlers[topic] = cb; },
}));
import { useMapRealtime } from "@/map/data/use-map-realtime";

test("a geofence event invalidates the geofences query", () => {
  renderHook(() => useMapRealtime());
  handlers["resource/geofence"]?.({ type: "updated" });
  expect(invalidate).toHaveBeenCalledWith({ queryKey: ["geo", "geofences"] });
});
```

> If Step 1 shows the real hook is not named `useSubscribe` or takes different args, update BOTH the mock and the implementation to match — keep them consistent.

- [ ] **Step 3: Run test to verify it fails**

Run: `bun run test -- use-map-realtime`
Expected: FAIL — module not found.

- [ ] **Step 4: Implement `use-map-realtime.ts`**

```ts
import { useQueryClient } from "@tanstack/react-query";
import { useSubscribe } from "@/components/realtime";

/** Live geofence/route deltas → refetch the map's GeoJSON layers. */
export function useMapRealtime() {
  const qc = useQueryClient();
  useSubscribe("resource/geofence", () => qc.invalidateQueries({ queryKey: ["geo", "geofences"] }));
  useSubscribe("resource/route", () => qc.invalidateQueries({ queryKey: ["geo", "routes"] }));
}
```
(Adjust `useSubscribe` import/usage to the real surface from Step 1.)

- [ ] **Step 5: Mount it in the canvas**

In `deck-canvas.tsx`, call `useMapRealtime()` near the top of `DeckCanvas` (one line). It owns its own subscription lifecycle.

- [ ] **Step 6: Run test + typecheck**

Run: `bun run test -- use-map-realtime` and `bun run typecheck`.
Expected: PASS / 0 errors.

- [ ] **Step 7: Commit**

```bash
git add apps/web/src/map/data/use-map-realtime.ts apps/web/src/map/data/use-map-realtime.test.ts apps/web/src/map/deck-canvas.tsx
git commit -m "feat(web/map): realtime geofence/route deltas refetch the map layers"
```

---

### Task 8: Phase-2 verification + live-verify checklist

**Files:** none (verification only).

- [ ] **Step 1: Full gate (parallel)**

Run together: `bun run typecheck`, `bun run test`, `bun run test:browser`, `bun run build`.
Expected: 0 type errors; all unit + browser suites pass; clean build.

- [ ] **Step 2: Live-verify against the real server**

Start koji (throwaway secret per the Phase-1 memory note) + `bun run dev`; on `/map`: draw a polygon (Polygon → click vertices → it appears), Save → it persists (confirm via `GET /api/v2/geofences?format=featurecollection` count increments and the realtime delta refetches the geofence layer live), Modify/Move an existing draft vertex, Merge two drawn polygons, drag the last-seen slider (markers re-query, network shows a new `/golbat-data` POST with the new `lastSeen`), switch spawnpoint TTH. Remember: `preview_screenshot` is BLACK (offscreen WebGL) — verify by DOM/network and the user's real tab.

- [ ] **Step 3: Update the ledger + memory pointer**

Append Phase-2 completion to `$(git rev-parse --git-path sdd)/progress.md` and update `koji-v2-map-phase1.md`'s "NEXT" line to point at Phase 3.

---

## Self-Review

**Spec coverage (Phase 2 = spec §8 P2):** drawing/editing ✓ (T1–T5) · save to resource ✓ (T4) · merge ✓ (T5) · filters (last-seen/TTH) ✓ (T6, server re-query — GPU `DataFilterExtension` correctly dropped because `/golbat-data` has no per-point attrs) · realtime marker/geofence deltas ✓ (T7). `HexagonLayer` (dense-marker heat) was an *optional* P2 item — deliberately deferred (YAGNI; add when a real density need appears). Route editing shares the same `EditableGeoJsonLayer`/Save path (geometry differs; the toolbar's Save targets geofence — a route-mode toggle is a small follow-up, noted not built).

**Zustand discipline:** all new state (drawMode/draftFeatures/selectedFeatureIndexes/filters) lives in the existing `mapUIStore`; every panel control subscribes to its own primitive (S3); `onEdit` writes via an action, not a render subscription; the 60fps camera is untouched (still transient). ✓

**Type consistency:** `DrawMode` defined once (T1) and used in store (T2), layers (T3), toolbar (T3). `BuildLayersInput.draft` shape matches what `deck-canvas` passes (T3). `fetchMarkers` new `tth` param matches the `useMarkers` + `<DeckCanvas>` call sites (T6). The realtime hook name is verified against the real surface before use (T7 Step 1). ✓

**Known soft spots (verify steps, not placeholders):** the exact realtime subscribe-hook signature — T7 Step 1 reads it before coding; turf 7 `union` input shape — T5 test pins it; editable-layers React-19/v9 runtime behavior — T3/T8 live-verify exercises it (fallback if a mode misbehaves: a thin custom draw handler on a plain `GeoJsonLayer`, but not expected given v9.3.7 compatibility).
