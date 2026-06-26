# Koji v2 deck.gl Map — Phase 1 (Read-Only Map) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship a full-bleed `/map` route in `apps/web` that renders Koji markers, geofences, routes, and S2 cells over a switchable MapLibre raster base via deck.gl v9 — read-only, no editing, no calc.

**Architecture:** MapLibre GL JS is the React root map; deck.gl layers ride on it as a `MapboxOverlay` (non-interleaved). The 60fps camera is uncontrolled and written to a transient zustand store (never through React render state). Server data lives in TanStack Query (already in the stack) via thin fetch hooks; only UI state + camera + persisted settings live in zustand. Pure logic (stores, coordinate math, style/layer factories, data hooks) is TDD'd in jsdom; the WebGL canvas + panels get browser smoke tests with deck/maplibre mocked.

**Tech Stack:** React 19, TypeScript 6, Vite 8, Tailwind v4, shadcn-admin-kit / ra-core 5.x, TanStack Query 5, zustand 5, deck.gl 9.3, MapLibre GL 4, react-map-gl 8.

## Global Constraints

- **Dependency versions (add exactly these floors):** `zustand@^5.0.8`, `maplibre-gl@^4.7.0`, `react-map-gl@^8.1.0`, `@deck.gl/core@^9.3.0`, `@deck.gl/react@^9.3.0`, `@deck.gl/layers@^9.3.0`, `@deck.gl/geo-layers@^9.3.0`, `@deck.gl/mapbox@^9.3.0`, `@deck.gl/extensions@^9.3.0`, `@deck.gl/aggregation-layers@^9.3.0`. Do NOT pin `luma.gl` (rides transitively; pinning it breaks shader compile). `@deck.gl-community/editable-layers` is **Phase 2** — do not add yet.
- **Zustand subscription discipline (hard requirement — per the project's measured benchmark):** subscribe to the **narrowest primitive** you need **in the component that uses it** (pattern S3). NEVER prop-drill a store slice. NEVER put `useStore` calls for many fields at the top of one container (pattern S4 — the worst). NEVER use `useShallow` over a wide slice fed to props (S2 — slower than naive). Derive counts/labels once (store state or one memoized selector), not in N subscribers. The live camera is **transient**: read it via `store.getState()` / `subscribeWithSelector` in an effect — NEVER via a `useStore(s => s.liveViewState)` hook in render.
- **Camera never re-renders React.** deck/MapLibre run uncontrolled (`initialViewState` + `onMove`/`onViewStateChange` → transient write). Never set controlled `viewState` and `initialViewState` together.
- **Coordinate rule (ONE adapter, `src/map/lib/coords.ts`):** Koji is `[lat, lon]` internally. The `/golbat-data` marker endpoint returns raw `[lat, lon]` pairs → **transpose to `[lng, lat]` for deck**. Geofence/route GeoJSON is already `[lng, lat]` → do NOT transpose. deck/MapLibre bounds are `[minLng, minLat, maxLng, maxLat]`. `lastSeen` is **seconds**.
- **Data surfaces:** `/golbat-data`, `/s2`, `/jobs` are public `/api/v2` (use the new `apiV2Fetch`, `credentials: "include"`). Geofence/route GeoJSON read is also `/api/v2` (`?format=featurecollection`). The existing `internalFetch`/dataProvider (`/internal`) is for resource CRUD only and has **no geometry** — do not use it for map geometry.
- **No v1 copy.** `apps/web-client/src/pages/map/**` may be consulted for parity ideas only; copy zero code.
- **Files stay focused** (CLAUDE.md split rule): one component/store/factory per file under `src/map/`.
- **Test commands:** unit (jsdom) `bun run test`; browser `bun run test:browser` (run FOREGROUND); types `bun run typecheck`; build `bun run build`. Unit tests are `*.test.ts(x)`, browser tests are `*.browser.test.tsx`.

---

## File Structure

```
apps/web/src/
  map/
    map-route.tsx                 # CustomRoute shell, full-bleed (Task 12)
    deck-canvas.tsx               # MapLibre root + MapboxOverlay(deck) + camera wiring (Task 10)
    stores/
      types.ts                    # LayerId, ViewState, MarkerCategory, Selection (Task 2)
      map-view-store.ts           # transient camera: live + throttled settled (Task 2)
      map-settings-store.ts       # persisted prefs: tileServerId, defaults, thresholds (Task 3)
      map-ui-store.ts             # layerVisibility, selection, hoverInfo (Task 4)
    lib/
      coords.ts                   # fromKojiLatLon, packMarkers, boundsToBboxArg (Task 5)
      map-style.ts                # tileServerUrl -> MapLibre StyleSpecification (Task 6)
      layers.ts                   # buildLayers(input) -> deck Layer[] (Task 9)
    data/
      use-markers.ts              # POST /api/v2/golbat-data/{category} by bbox (Task 7)
      use-geo-features.ts         # GET /api/v2/{geofences,routes}?format=featurecollection (Task 8)
      use-s2-cells.ts             # POST /api/v2/s2/{level} by bbox (Task 8)
    panels/
      layer-drawer.tsx            # per-layer on/off (Task 11)
      tile-server-select.tsx      # base-map switch (Task 11)
      coordinate-readout.tsx      # settled lng/lat/zoom HUD (Task 11)
      selection-popup.tsx         # clicked-feature card (Task 11)
  lib/http.ts                     # add apiV2Fetch + API_V2_BASE (Task 1)
  App.tsx                         # register /map CustomRoute (Task 1 skeleton, Task 12 real)
```

---

### Task 1: Dependencies, `apiV2Fetch`, `/map` route skeleton

**Files:**
- Modify: `apps/web/package.json` (deps)
- Modify: `apps/web/src/lib/http.ts` (add `API_V2_BASE`, `apiV2Fetch`)
- Create: `apps/web/src/lib/http.api-v2.test.ts`
- Create: `apps/web/src/map/map-route.tsx` (placeholder)
- Modify: `apps/web/src/App.tsx` (register route)

**Interfaces:**
- Produces: `apiV2Fetch(path: string, init?: RequestInit): Promise<{ status: number; json: unknown }>` — same shape as `internalFetch` but base `/api/v2`. Reuses `unwrapResponse<T>` / `HttpError` from `http.ts`.

- [ ] **Step 1: Add dependencies**

```bash
cd apps/web && bun add zustand@^5.0.8 maplibre-gl@^4.7.0 react-map-gl@^8.1.0 \
  @deck.gl/core@^9.3.0 @deck.gl/react@^9.3.0 @deck.gl/layers@^9.3.0 \
  @deck.gl/geo-layers@^9.3.0 @deck.gl/mapbox@^9.3.0 @deck.gl/extensions@^9.3.0 \
  @deck.gl/aggregation-layers@^9.3.0
```

Then commit the `package.json` + `bun.lock` together (lockfile discipline).

- [ ] **Step 2: Write the failing test for `apiV2Fetch`**

`apps/web/src/lib/http.api-v2.test.ts`:

```ts
import { afterEach, expect, test, vi } from "vitest";
import { apiV2Fetch } from "@/lib/http";

afterEach(() => vi.restoreAllMocks());

test("apiV2Fetch hits the /api/v2 base with credentials + JSON headers", async () => {
  const fetchMock = vi.fn().mockResolvedValue(
    new Response(JSON.stringify({ status: "ok", data: { ok: true } }), { status: 200 }),
  );
  vi.stubGlobal("fetch", fetchMock);

  const res = await apiV2Fetch("/health");

  expect(fetchMock).toHaveBeenCalledWith(
    "/api/v2/health",
    expect.objectContaining({ credentials: "include" }),
  );
  expect(res.status).toBe(200);
  expect(res.json).toEqual({ status: "ok", data: { ok: true } });
});

test("apiV2Fetch returns null json for an empty body", async () => {
  vi.stubGlobal("fetch", vi.fn().mockResolvedValue(new Response("", { status: 204 })));
  const res = await apiV2Fetch("/x", { method: "POST" });
  expect(res.json).toBeNull();
});
```

- [ ] **Step 3: Run test to verify it fails**

Run: `bun run test -- http.api-v2`
Expected: FAIL — `apiV2Fetch` is not exported.

- [ ] **Step 4: Implement `apiV2Fetch`**

Add to `apps/web/src/lib/http.ts` (mirror `internalFetch`, different base):

```ts
export const API_V2_BASE = "/api/v2";

/** Like `internalFetch`, but targets the public `/api/v2` surface (markers,
 *  s2, jobs, GeoJSON reads). Same session-cookie auth (`credentials: include`). */
export async function apiV2Fetch(
  path: string,
  init?: RequestInit,
): Promise<{ status: number; json: unknown }> {
  const res = await fetch(`${API_V2_BASE}${path}`, {
    credentials: "include",
    ...init,
    headers: {
      "Content-Type": "application/json",
      Accept: "application/json",
      ...(init?.headers ?? {}),
    },
  });
  const text = await res.text();
  const json = text ? JSON.parse(text) : null;
  return { status: res.status, json };
}
```

- [ ] **Step 5: Run test to verify it passes**

Run: `bun run test -- http.api-v2`
Expected: PASS (2 tests).

- [ ] **Step 6: Add the placeholder route + registration**

`apps/web/src/map/map-route.tsx`:

```tsx
/** Phase 1 placeholder — replaced by the full map in Task 12. */
export function MapRoute() {
  return <div className="grid h-screen w-screen place-items-center">Map (Phase 1 WIP)</div>;
}
```

In `apps/web/src/App.tsx`, add the import and a second `<Route>` inside the existing `<CustomRoutes>`:

```tsx
import { MapRoute } from "@/map/map-route";
// ...
<CustomRoutes>
  <Route element={<ImportWizard />} path="/import" />
  <Route element={<MapRoute />} path="/map" />
</CustomRoutes>
```

- [ ] **Step 7: Verify it compiles and the route mounts**

Run: `bun run typecheck` → Expected: 0 errors.
Run: `bun run test -- http.api-v2` → Expected: PASS.

- [ ] **Step 8: Commit**

```bash
git add apps/web/package.json apps/web/bun.lock apps/web/src/lib/http.ts \
  apps/web/src/lib/http.api-v2.test.ts apps/web/src/map/map-route.tsx apps/web/src/App.tsx
git commit -m "feat(web/map): deck.gl deps + apiV2Fetch + /map route skeleton"
```

---

### Task 2: Store types + transient camera store

**Files:**
- Create: `apps/web/src/map/stores/types.ts`
- Create: `apps/web/src/map/stores/map-view-store.ts`
- Create: `apps/web/src/map/stores/map-view-store.test.ts`

**Interfaces:**
- Produces (`types.ts`):
  ```ts
  export type LayerId = "gyms" | "pokestops" | "spawnpoints" | "stations" | "geofences" | "routes" | "s2";
  export type MarkerCategory = "gym" | "pokestop" | "spawnpoint" | "station" | "fort";
  export interface ViewState { longitude: number; latitude: number; zoom: number; pitch: number; bearing: number; }
  /** deck bounds order: [minLng, minLat, maxLng, maxLat] */
  export type Bounds = [number, number, number, number];
  export interface Selection { kind: "marker" | "geofence" | "route" | null; id: string | null; }
  ```
- Produces (`map-view-store.ts`): `useMapViewStore` (zustand store with `subscribeWithSelector`), fields `liveViewState: ViewState`, `settledViewState: ViewState`, `settledBounds: Bounds`, actions `setLive(v: ViewState, bounds: Bounds): void` (hot path), `flushSettle(): void` (writes settled = live; called by a throttle). The store factory `createMapViewStore(throttleMs?)` is exported for tests.

- [ ] **Step 1: Write `types.ts`** (no test needed — pure type aliases)

Create `apps/web/src/map/stores/types.ts` with the four exports above.

- [ ] **Step 2: Write the failing test for the camera store**

`apps/web/src/map/stores/map-view-store.test.ts`:

```ts
import { beforeEach, afterEach, expect, test, vi } from "vitest";
import { createMapViewStore } from "@/map/stores/map-view-store";
import type { ViewState, Bounds } from "@/map/stores/types";

const VS: ViewState = { longitude: 1, latitude: 2, zoom: 10, pitch: 0, bearing: 0 };
const B: Bounds = [0, 0, 2, 4];

beforeEach(() => vi.useFakeTimers());
afterEach(() => vi.useRealTimers());

test("setLive updates liveViewState immediately but NOT settled (until throttle fires)", () => {
  const store = createMapViewStore(200);
  store.getState().setLive(VS, B);
  expect(store.getState().liveViewState).toEqual(VS);
  // settled still at defaults right after a single setLive
  expect(store.getState().settledBounds).not.toEqual(B);
  vi.advanceTimersByTime(200);
  expect(store.getState().settledViewState).toEqual(VS);
  expect(store.getState().settledBounds).toEqual(B);
});

test("rapid setLive calls notify settled subscribers at most once per window", () => {
  const store = createMapViewStore(200);
  const settledSpy = vi.fn();
  // subscribe ONLY to settledBounds (subscribeWithSelector)
  store.subscribe((s) => s.settledBounds, settledSpy);
  for (let i = 0; i < 10; i++) store.getState().setLive({ ...VS, zoom: 10 + i }, B);
  expect(settledSpy).not.toHaveBeenCalled(); // throttled, not yet flushed
  vi.advanceTimersByTime(200);
  expect(settledSpy).toHaveBeenCalledTimes(1);
});
```

- [ ] **Step 3: Run test to verify it fails**

Run: `bun run test -- map-view-store`
Expected: FAIL — module not found.

- [ ] **Step 4: Implement the camera store**

`apps/web/src/map/stores/map-view-store.ts`:

```ts
import { create } from "zustand";
import { subscribeWithSelector } from "zustand/middleware";
import type { Bounds, ViewState } from "@/map/stores/types";

const DEFAULT_VIEW: ViewState = { longitude: 0, latitude: 0, zoom: 2, pitch: 0, bearing: 0 };
const DEFAULT_BOUNDS: Bounds = [-180, -85, 180, 85];

export interface MapViewState {
  /** Hot path — written ~60fps. Read transiently (getState / subscribe in effect), NEVER via useStore in render. */
  liveViewState: ViewState;
  /** Throttled snapshot — the ONLY camera value safe to subscribe to in render. */
  settledViewState: ViewState;
  settledBounds: Bounds;
  setLive: (v: ViewState, bounds: Bounds) => void;
  flushSettle: () => void;
}

export function createMapViewStore(throttleMs = 200) {
  let pending: { v: ViewState; b: Bounds } | null = null;
  let timer: ReturnType<typeof setTimeout> | null = null;

  return create<MapViewState>()(
    subscribeWithSelector((set, get) => ({
      liveViewState: DEFAULT_VIEW,
      settledViewState: DEFAULT_VIEW,
      settledBounds: DEFAULT_BOUNDS,
      setLive: (v, b) => {
        // liveViewState write does not cause renders because no component
        // subscribes to it via a hook (it is read transiently only).
        set({ liveViewState: v });
        pending = { v, b };
        if (timer == null) {
          timer = setTimeout(() => {
            timer = null;
            get().flushSettle();
          }, throttleMs);
        }
      },
      flushSettle: () => {
        if (!pending) return;
        set({ settledViewState: pending.v, settledBounds: pending.b });
        pending = null;
      },
    })),
  );
}

export const useMapViewStore = createMapViewStore();
```

- [ ] **Step 5: Run test to verify it passes**

Run: `bun run test -- map-view-store`
Expected: PASS (2 tests).

- [ ] **Step 6: Commit**

```bash
git add apps/web/src/map/stores/types.ts apps/web/src/map/stores/map-view-store.ts \
  apps/web/src/map/stores/map-view-store.test.ts
git commit -m "feat(web/map): store types + transient throttled camera store"
```

---

### Task 3: Persisted settings store

**Files:**
- Create: `apps/web/src/map/stores/map-settings-store.ts`
- Create: `apps/web/src/map/stores/map-settings-store.test.ts`

**Interfaces:**
- Produces: `useMapSettingsStore` with `tileServerId: string`, `defaultLayerVisibility: Record<LayerId, boolean>`, `areaThresholds: { gym: number; pokestop: number; spawnpoint: number }`, `markerRadius: number`, actions `setTileServerId(id: string)`, `setMarkerRadius(n: number)`. Persisted to `localStorage` key `koji-map-settings`. Only these fields are persisted (nothing transient).

- [ ] **Step 1: Write the failing test**

`apps/web/src/map/stores/map-settings-store.test.ts`:

```ts
import { beforeEach, expect, test } from "vitest";
import { useMapSettingsStore } from "@/map/stores/map-settings-store";

beforeEach(() => {
  localStorage.clear();
  useMapSettingsStore.setState({ tileServerId: "default", markerRadius: 30 });
});

test("defaults are present", () => {
  const s = useMapSettingsStore.getState();
  expect(s.tileServerId).toBe("default");
  expect(s.defaultLayerVisibility.geofences).toBe(true);
  expect(s.areaThresholds.pokestop).toBeGreaterThan(0);
});

test("setTileServerId persists to localStorage under koji-map-settings", () => {
  useMapSettingsStore.getState().setTileServerId("osm-bright");
  expect(useMapSettingsStore.getState().tileServerId).toBe("osm-bright");
  const raw = localStorage.getItem("koji-map-settings");
  expect(raw).toContain("osm-bright");
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `bun run test -- map-settings-store`
Expected: FAIL — module not found.

- [ ] **Step 3: Implement the persisted store**

`apps/web/src/map/stores/map-settings-store.ts`:

```ts
import { create } from "zustand";
import { persist } from "zustand/middleware";
import type { LayerId } from "@/map/stores/types";

const DEFAULT_VISIBILITY: Record<LayerId, boolean> = {
  gyms: false, pokestops: false, spawnpoints: false, stations: false,
  geofences: true, routes: true, s2: false,
};

export interface MapSettingsState {
  tileServerId: string;
  defaultLayerVisibility: Record<LayerId, boolean>;
  areaThresholds: { gym: number; pokestop: number; spawnpoint: number };
  markerRadius: number;
  setTileServerId: (id: string) => void;
  setMarkerRadius: (n: number) => void;
}

export const useMapSettingsStore = create<MapSettingsState>()(
  persist(
    (set) => ({
      tileServerId: "default",
      defaultLayerVisibility: DEFAULT_VISIBILITY,
      areaThresholds: { gym: 100, pokestop: 100, spawnpoint: 100 },
      markerRadius: 30,
      setTileServerId: (id) => set({ tileServerId: id }),
      setMarkerRadius: (n) => set({ markerRadius: n }),
    }),
    {
      name: "koji-map-settings",
      // persist EVERYTHING here is fine — this store holds only long-lived prefs.
      // transient camera/filters/selection live in OTHER stores and are never persisted.
    },
  ),
);
```

- [ ] **Step 4: Run test to verify it passes**

Run: `bun run test -- map-settings-store`
Expected: PASS (2 tests).

- [ ] **Step 5: Commit**

```bash
git add apps/web/src/map/stores/map-settings-store.ts apps/web/src/map/stores/map-settings-store.test.ts
git commit -m "feat(web/map): persisted map settings store"
```

---

### Task 4: UI store (layer visibility, selection, hover)

**Files:**
- Create: `apps/web/src/map/stores/map-ui-store.ts`
- Create: `apps/web/src/map/stores/map-ui-store.test.ts`

**Interfaces:**
- Produces: `useMapUIStore` with `layerVisibility: Record<LayerId, boolean>` (seeded from settings' defaults at init), `selection: Selection`, `hoverInfo: { x: number; y: number; id: string | null } | null`, `s2Level: number`, actions `toggleLayer(id: LayerId)`, `setSelection(sel: Selection)`, `setHover(h)`, `setS2Level(n)`.

- [ ] **Step 1: Write the failing test**

`apps/web/src/map/stores/map-ui-store.test.ts`:

```ts
import { beforeEach, expect, test } from "vitest";
import { useMapUIStore } from "@/map/stores/map-ui-store";

beforeEach(() => useMapUIStore.setState(useMapUIStore.getInitialState()));

test("toggleLayer flips exactly one layer and leaves others untouched", () => {
  const before = useMapUIStore.getState().layerVisibility.gyms;
  useMapUIStore.getState().toggleLayer("gyms");
  expect(useMapUIStore.getState().layerVisibility.gyms).toBe(!before);
  expect(useMapUIStore.getState().layerVisibility.geofences).toBe(true);
});

test("setSelection stores kind + id", () => {
  useMapUIStore.getState().setSelection({ kind: "geofence", id: "42" });
  expect(useMapUIStore.getState().selection).toEqual({ kind: "geofence", id: "42" });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `bun run test -- map-ui-store`
Expected: FAIL — module not found.

- [ ] **Step 3: Implement the UI store**

`apps/web/src/map/stores/map-ui-store.ts`:

```ts
import { create } from "zustand";
import type { LayerId, Selection } from "@/map/stores/types";

interface HoverInfo { x: number; y: number; id: string | null; }

export interface MapUIState {
  layerVisibility: Record<LayerId, boolean>;
  selection: Selection;
  hoverInfo: HoverInfo | null;
  s2Level: number;
  toggleLayer: (id: LayerId) => void;
  setSelection: (sel: Selection) => void;
  setHover: (h: HoverInfo | null) => void;
  setS2Level: (n: number) => void;
}

const INITIAL_VISIBILITY: Record<LayerId, boolean> = {
  gyms: false, pokestops: false, spawnpoints: false, stations: false,
  geofences: true, routes: true, s2: false,
};

export const useMapUIStore = create<MapUIState>()((set) => ({
  layerVisibility: INITIAL_VISIBILITY,
  selection: { kind: null, id: null },
  hoverInfo: null,
  s2Level: 15,
  toggleLayer: (id) =>
    set((s) => ({ layerVisibility: { ...s.layerVisibility, [id]: !s.layerVisibility[id] } })),
  setSelection: (sel) => set({ selection: sel }),
  setHover: (h) => set({ hoverInfo: h }),
  setS2Level: (n) => set({ s2Level: n }),
}));
```

- [ ] **Step 4: Run test to verify it passes**

Run: `bun run test -- map-ui-store`
Expected: PASS (2 tests). (`getInitialState()` is a built-in zustand v5 store method.)

- [ ] **Step 5: Commit**

```bash
git add apps/web/src/map/stores/map-ui-store.ts apps/web/src/map/stores/map-ui-store.test.ts
git commit -m "feat(web/map): map UI store (layer visibility, selection, hover)"
```

---

### Task 5: Coordinate adapter (`coords.ts`)

**Files:**
- Create: `apps/web/src/map/lib/coords.ts`
- Create: `apps/web/src/map/lib/coords.test.ts`

**Interfaces:**
- Produces:
  - `fromKojiLatLon(pair: [number, number]): [number, number]` — `[lat, lon] -> [lng, lat]`.
  - `packMarkers(points: [number, number][]): Float32Array` — Koji `[lat,lon]` pairs → flat `[lng, lat, lng, lat, ...]` for deck binary accessors (stride 2).
  - `boundsToBboxArg(b: Bounds): { min_lat: number; min_lon: number; max_lat: number; max_lon: number }` — deck `[minLng,minLat,maxLng,maxLat]` → Koji snake_case bbox (lat/lon).

- [ ] **Step 1: Write the failing test**

`apps/web/src/map/lib/coords.test.ts`:

```ts
import { expect, test } from "vitest";
import { boundsToBboxArg, fromKojiLatLon, packMarkers } from "@/map/lib/coords";
import type { Bounds } from "@/map/stores/types";

test("fromKojiLatLon transposes [lat,lon] -> [lng,lat]", () => {
  expect(fromKojiLatLon([47.5, -122.3])).toEqual([-122.3, 47.5]);
});

test("packMarkers produces a stride-2 [lng,lat,...] Float32Array", () => {
  const out = packMarkers([[47.5, -122.3], [10, 20]]);
  expect(out).toBeInstanceOf(Float32Array);
  expect(out.length).toBe(4);
  expect(Array.from(out)).toEqual([-122.3, 47.5, 20, 10].map((n) => Math.fround(n)));
});

test("boundsToBboxArg maps [minLng,minLat,maxLng,maxLat] to Koji lat/lon snake_case", () => {
  const b: Bounds = [-122.4, 47.4, -122.2, 47.6];
  expect(boundsToBboxArg(b)).toEqual({
    min_lat: 47.4, min_lon: -122.4, max_lat: 47.6, max_lon: -122.2,
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `bun run test -- coords`
Expected: FAIL — module not found.

- [ ] **Step 3: Implement `coords.ts`**

```ts
import type { Bounds } from "@/map/stores/types";

/** Koji marker pairs are [lat, lon]; deck/GeoJSON want [lng, lat]. */
export function fromKojiLatLon(pair: [number, number]): [number, number] {
  return [pair[1], pair[0]];
}

/** Flatten Koji [lat,lon] pairs into a [lng,lat,...] Float32Array for deck binary accessors. */
export function packMarkers(points: [number, number][]): Float32Array {
  const out = new Float32Array(points.length * 2);
  for (let i = 0; i < points.length; i++) {
    out[i * 2] = points[i][1]; // lng
    out[i * 2 + 1] = points[i][0]; // lat
  }
  return out;
}

export function boundsToBboxArg(
  b: Bounds,
): { min_lat: number; min_lon: number; max_lat: number; max_lon: number } {
  return { min_lon: b[0], min_lat: b[1], max_lon: b[2], max_lat: b[3] };
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `bun run test -- coords`
Expected: PASS (3 tests).

- [ ] **Step 5: Commit**

```bash
git add apps/web/src/map/lib/coords.ts apps/web/src/map/lib/coords.test.ts
git commit -m "feat(web/map): coordinate adapter (transpose, pack, bbox)"
```

---

### Task 6: MapLibre style factory (`map-style.ts`)

**Files:**
- Create: `apps/web/src/map/lib/map-style.ts`
- Create: `apps/web/src/map/lib/map-style.test.ts`

**Interfaces:**
- Produces: `rasterStyle(tileUrl: string): StyleSpecification` — a MapLibre style.json with one raster source (`tiles: [tileUrl]`, `tileSize: 256`) and one raster layer covering it. `tileUrl` is an XYZ template (`https://.../{z}/{x}/{y}.png`).

- [ ] **Step 1: Write the failing test**

`apps/web/src/map/lib/map-style.test.ts`:

```ts
import { expect, test } from "vitest";
import { rasterStyle } from "@/map/lib/map-style";

test("rasterStyle wraps an XYZ url in a single raster source + layer", () => {
  const style = rasterStyle("https://tile.example/{z}/{x}/{y}.png");
  expect(style.version).toBe(8);
  const src = style.sources.osm as { type: string; tiles: string[]; tileSize: number };
  expect(src.type).toBe("raster");
  expect(src.tiles).toEqual(["https://tile.example/{z}/{x}/{y}.png"]);
  expect(style.layers).toHaveLength(1);
  expect(style.layers[0]).toMatchObject({ type: "raster", source: "osm" });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `bun run test -- map-style`
Expected: FAIL — module not found.

- [ ] **Step 3: Implement `map-style.ts`**

```ts
import type { StyleSpecification } from "maplibre-gl";

/** Build a minimal MapLibre style from a single XYZ raster tile template.
 *  Switching tile servers = calling this with a new url and swapping mapStyle. */
export function rasterStyle(tileUrl: string): StyleSpecification {
  return {
    version: 8,
    sources: {
      osm: { type: "raster", tiles: [tileUrl], tileSize: 256, attribution: "" },
    },
    layers: [{ id: "osm", type: "raster", source: "osm" }],
  };
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `bun run test -- map-style`
Expected: PASS (1 test).

- [ ] **Step 5: Commit**

```bash
git add apps/web/src/map/lib/map-style.ts apps/web/src/map/lib/map-style.test.ts
git commit -m "feat(web/map): MapLibre raster style factory"
```

---

### Task 7: Marker data hook (`use-markers.ts`)

**Files:**
- Create: `apps/web/src/map/data/use-markers.ts`
- Create: `apps/web/src/map/data/use-markers.test.ts`

**Interfaces:**
- Consumes: `apiV2Fetch` (Task 1), `boundsToBboxArg` (Task 5), `Bounds`/`MarkerCategory` (Task 2).
- Produces:
  - `fetchMarkers(category: MarkerCategory, bounds: Bounds, lastSeen: number): Promise<[number, number][]>` — POSTs `/golbat-data/{category}` with `{ bbox }`, returns Koji `[lat,lon]` pairs. (Pure-ish; unwraps `{points}` whether enveloped or raw.)
  - `useMarkers(category, bounds, lastSeen, enabled)` — TanStack Query wrapper keyed by `["markers", category, bounds, lastSeen]`.

- [ ] **Step 1: Write the failing test for `fetchMarkers`**

`apps/web/src/map/data/use-markers.test.ts`:

```ts
import { afterEach, expect, test, vi } from "vitest";
import { fetchMarkers } from "@/map/data/use-markers";
import type { Bounds } from "@/map/stores/types";

afterEach(() => vi.restoreAllMocks());
const B: Bounds = [-122.4, 47.4, -122.2, 47.6];

test("fetchMarkers POSTs the bbox and returns [lat,lon] pairs (enveloped)", async () => {
  const fetchMock = vi.fn().mockResolvedValue(
    new Response(JSON.stringify({ status: "ok", data: { points: [[47.5, -122.3]] } }), { status: 200 }),
  );
  vi.stubGlobal("fetch", fetchMock);

  const pts = await fetchMarkers("pokestop", B, 0);

  const [url, init] = fetchMock.mock.calls[0];
  expect(url).toBe("/api/v2/golbat-data/pokestop");
  expect(init.method).toBe("POST");
  expect(JSON.parse(init.body).bbox).toMatchObject({ min_lat: 47.4, max_lon: -122.2 });
  expect(pts).toEqual([[47.5, -122.3]]);
});

test("fetchMarkers also accepts a raw (non-enveloped) {points} body", async () => {
  vi.stubGlobal("fetch", vi.fn().mockResolvedValue(
    new Response(JSON.stringify({ points: [[1, 2]] }), { status: 200 }),
  ));
  expect(await fetchMarkers("gym", B, 0)).toEqual([[1, 2]]);
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `bun run test -- use-markers`
Expected: FAIL — module not found.

- [ ] **Step 3: Implement `use-markers.ts`**

```ts
import { useQuery } from "@tanstack/react-query";
import { apiV2Fetch } from "@/lib/http";
import { boundsToBboxArg } from "@/map/lib/coords";
import type { Bounds, MarkerCategory } from "@/map/stores/types";

interface MarkersBody { points: [number, number][] }

/** Accept either `{status,data:{points}}` (enveloped) or raw `{points}`. */
function readPoints(json: unknown): [number, number][] {
  const j = json as { data?: MarkersBody; points?: [number, number][] };
  return j?.data?.points ?? j?.points ?? [];
}

export async function fetchMarkers(
  category: MarkerCategory,
  bounds: Bounds,
  lastSeen: number,
): Promise<[number, number][]> {
  const res = await apiV2Fetch(`/golbat-data/${category}`, {
    method: "POST",
    body: JSON.stringify({ bbox: boundsToBboxArg(bounds), lastSeen }),
  });
  if (res.status < 200 || res.status >= 300) return [];
  return readPoints(res.json);
}

export function useMarkers(
  category: MarkerCategory,
  bounds: Bounds,
  lastSeen: number,
  enabled: boolean,
) {
  return useQuery({
    queryKey: ["markers", category, bounds, lastSeen],
    queryFn: () => fetchMarkers(category, bounds, lastSeen),
    enabled,
    staleTime: 30_000,
  });
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `bun run test -- use-markers`
Expected: PASS (2 tests).

- [ ] **Step 5: Verify the real endpoint path + body shape**

Run: `grep -rn "golbat-data" ../../crates/koji-service/src/public/v2/golbat_data.rs | head` and confirm the POST route is `/golbat-data/{category}` and the body field is `bbox` (vs `area`). If the field name differs, fix `fetchMarkers` + its test, re-run Step 4.

- [ ] **Step 6: Commit**

```bash
git add apps/web/src/map/data/use-markers.ts apps/web/src/map/data/use-markers.test.ts
git commit -m "feat(web/map): viewport-bbox marker fetch hook"
```

---

### Task 8: Geo-feature + S2 data hooks

**Files:**
- Create: `apps/web/src/map/data/use-geo-features.ts`
- Create: `apps/web/src/map/data/use-s2-cells.ts`
- Create: `apps/web/src/map/data/use-geo-features.test.ts`
- Create: `apps/web/src/map/data/use-s2-cells.test.ts`

**Interfaces:**
- Consumes: `apiV2Fetch` (Task 1), `boundsToBboxArg` (Task 5).
- Produces:
  - `fetchFeatureCollection(resource: "geofences" | "routes"): Promise<GeoJSON.FeatureCollection>` + `useGeoFeatures(resource, enabled)`.
  - `fetchS2Cells(level: number, bounds: Bounds): Promise<string[]>` (returns cell-id strings) + `useS2Cells(level, bounds, enabled)`.

- [ ] **Step 1: Write failing tests**

`apps/web/src/map/data/use-geo-features.test.ts`:

```ts
import { afterEach, expect, test, vi } from "vitest";
import { fetchFeatureCollection } from "@/map/data/use-geo-features";

afterEach(() => vi.restoreAllMocks());

test("fetchFeatureCollection requests featurecollection format and returns it", async () => {
  const fc = { type: "FeatureCollection", features: [] };
  const fetchMock = vi.fn().mockResolvedValue(
    new Response(JSON.stringify({ status: "ok", data: fc }), { status: 200 }),
  );
  vi.stubGlobal("fetch", fetchMock);
  const out = await fetchFeatureCollection("geofences");
  expect(fetchMock.mock.calls[0][0]).toBe("/api/v2/geofences?format=featurecollection");
  expect(out).toEqual(fc);
});
```

`apps/web/src/map/data/use-s2-cells.test.ts`:

```ts
import { afterEach, expect, test, vi } from "vitest";
import { fetchS2Cells } from "@/map/data/use-s2-cells";
import type { Bounds } from "@/map/stores/types";

afterEach(() => vi.restoreAllMocks());
const B: Bounds = [-122.4, 47.4, -122.2, 47.6];

test("fetchS2Cells POSTs /s2/{level} with bbox and returns cell-id strings", async () => {
  const fetchMock = vi.fn().mockResolvedValue(
    new Response(JSON.stringify({ status: "ok", data: [{ id: "abc" }, { id: "def" }] }), { status: 200 }),
  );
  vi.stubGlobal("fetch", fetchMock);
  const ids = await fetchS2Cells(15, B);
  expect(fetchMock.mock.calls[0][0]).toBe("/api/v2/s2/15");
  expect(JSON.parse(fetchMock.mock.calls[0][1].body)).toMatchObject({ min_lat: 47.4 });
  expect(ids).toEqual(["abc", "def"]);
});
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `bun run test -- use-geo-features use-s2-cells`
Expected: FAIL — modules not found.

- [ ] **Step 3: Implement both hooks**

`apps/web/src/map/data/use-geo-features.ts`:

```ts
import { useQuery } from "@tanstack/react-query";
import { apiV2Fetch } from "@/lib/http";

const EMPTY: GeoJSON.FeatureCollection = { type: "FeatureCollection", features: [] };

export async function fetchFeatureCollection(
  resource: "geofences" | "routes",
): Promise<GeoJSON.FeatureCollection> {
  const res = await apiV2Fetch(`/${resource}?format=featurecollection`);
  if (res.status < 200 || res.status >= 300) return EMPTY;
  const j = res.json as { data?: GeoJSON.FeatureCollection; type?: string };
  return j?.data ?? (j?.type === "FeatureCollection" ? (j as GeoJSON.FeatureCollection) : EMPTY);
}

export function useGeoFeatures(resource: "geofences" | "routes", enabled: boolean) {
  return useQuery({
    queryKey: ["geo", resource],
    queryFn: () => fetchFeatureCollection(resource),
    enabled,
    staleTime: 60_000,
  });
}
```

`apps/web/src/map/data/use-s2-cells.ts`:

```ts
import { useQuery } from "@tanstack/react-query";
import { apiV2Fetch } from "@/lib/http";
import { boundsToBboxArg } from "@/map/lib/coords";
import type { Bounds } from "@/map/stores/types";

export async function fetchS2Cells(level: number, bounds: Bounds): Promise<string[]> {
  const res = await apiV2Fetch(`/s2/${level}`, {
    method: "POST",
    body: JSON.stringify(boundsToBboxArg(bounds)),
  });
  if (res.status < 200 || res.status >= 300) return [];
  const j = res.json as { data?: { id: string }[]; } | { id: string }[];
  const cells = Array.isArray(j) ? j : (j.data ?? []);
  return cells.map((c) => c.id);
}

export function useS2Cells(level: number, bounds: Bounds, enabled: boolean) {
  return useQuery({
    queryKey: ["s2", level, bounds],
    queryFn: () => fetchS2Cells(level, bounds),
    enabled,
    staleTime: 60_000,
  });
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `bun run test -- use-geo-features use-s2-cells`
Expected: PASS (2 tests).

- [ ] **Step 5: Verify the real S2 body shape**

Run: `grep -rn "BoundsArg\|min_lat\|pub async fn" ../../crates/koji-service/src/public/v2/s2.rs | head` and confirm `POST /s2/{level}` takes the snake_case bbox + returns objects with an `id` string. Adjust if the response key differs (e.g. `token`), re-run Step 4.

- [ ] **Step 6: Commit**

```bash
git add apps/web/src/map/data/use-geo-features.ts apps/web/src/map/data/use-s2-cells.ts \
  apps/web/src/map/data/use-geo-features.test.ts apps/web/src/map/data/use-s2-cells.test.ts
git commit -m "feat(web/map): geofence/route GeoJSON + S2 cell data hooks"
```

---

### Task 9: Layer factory (`layers.ts`)

**Files:**
- Create: `apps/web/src/map/lib/layers.ts`
- Create: `apps/web/src/map/lib/layers.test.ts`

**Interfaces:**
- Consumes: `LayerId` (Task 2), `packMarkers` (Task 5).
- Produces: `buildLayers(input: BuildLayersInput): Layer[]` where
  ```ts
  interface MarkerSet { id: LayerId; points: [number, number][]; color: [number, number, number]; }
  interface BuildLayersInput {
    visibility: Record<LayerId, boolean>;
    markerSets: MarkerSet[];        // gyms/pokestops/spawnpoints/stations
    geofences: GeoJSON.FeatureCollection;
    routes: GeoJSON.FeatureCollection;
    s2CellIds: string[];
    markerRadius: number;
    onClick: (info: { layer?: { id?: string }; object?: unknown; index: number }) => void;
  }
  ```
  Returns deck layers whose `visible` prop reflects `visibility`. The factory is **pure** — it constructs layer instances from inputs and never reads a store; the canvas calls it with values it selected.

- [ ] **Step 1: Write the failing test**

`apps/web/src/map/lib/layers.test.ts`:

```ts
import { expect, test, vi } from "vitest";
import { buildLayers } from "@/map/lib/layers";
import type { LayerId } from "@/map/stores/types";

const vis = (over: Partial<Record<LayerId, boolean>> = {}): Record<LayerId, boolean> => ({
  gyms: false, pokestops: true, spawnpoints: false, stations: false,
  geofences: true, routes: false, s2: false, ...over,
});
const EMPTY: GeoJSON.FeatureCollection = { type: "FeatureCollection", features: [] };

test("buildLayers emits one visible scatter layer per visible marker set", () => {
  const layers = buildLayers({
    visibility: vis(),
    markerSets: [{ id: "pokestops", points: [[47.5, -122.3]], color: [0, 120, 255] }],
    geofences: EMPTY, routes: EMPTY, s2CellIds: [],
    markerRadius: 30, onClick: vi.fn(),
  });
  const stop = layers.find((l) => l.id === "markers-pokestops");
  expect(stop).toBeDefined();
  expect(stop!.props.visible).toBe(true);
  // geofences visible, routes hidden
  expect(layers.find((l) => l.id === "geofences")!.props.visible).toBe(true);
  expect(layers.find((l) => l.id === "routes")!.props.visible).toBe(false);
});

test("S2 layer is present but hidden when s2 visibility is off", () => {
  const layers = buildLayers({
    visibility: vis({ s2: false }), markerSets: [],
    geofences: EMPTY, routes: EMPTY, s2CellIds: ["abc"],
    markerRadius: 30, onClick: vi.fn(),
  });
  expect(layers.find((l) => l.id === "s2")!.props.visible).toBe(false);
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `bun run test -- layers`
Expected: FAIL — module not found.

- [ ] **Step 3: Implement `layers.ts`**

```ts
import { ScatterplotLayer, GeoJsonLayer } from "@deck.gl/layers";
import { S2Layer } from "@deck.gl/geo-layers";
import type { Layer } from "@deck.gl/core";
import type { LayerId } from "@/map/stores/types";
import { packMarkers } from "@/map/lib/coords";

interface MarkerSet { id: LayerId; points: [number, number][]; color: [number, number, number]; }
export interface BuildLayersInput {
  visibility: Record<LayerId, boolean>;
  markerSets: MarkerSet[];
  geofences: GeoJSON.FeatureCollection;
  routes: GeoJSON.FeatureCollection;
  s2CellIds: string[];
  markerRadius: number;
  onClick: (info: { layer?: { id?: string }; object?: unknown; index: number }) => void;
}

export function buildLayers(input: BuildLayersInput): Layer[] {
  const { visibility, markerSets, geofences, routes, s2CellIds, markerRadius, onClick } = input;

  const markerLayers = markerSets.map((set) => {
    const positions = packMarkers(set.points);
    return new ScatterplotLayer({
      id: `markers-${set.id}`,
      visible: visibility[set.id],
      data: { length: set.points.length, attributes: { getPosition: { value: positions, size: 2 } } },
      getRadius: markerRadius,
      radiusUnits: "meters",
      radiusMinPixels: 2,
      getFillColor: [...set.color, 200],
      pickable: true,
      onClick,
    });
  });

  return [
    new GeoJsonLayer({
      id: "geofences", visible: visibility.geofences, data: geofences,
      filled: true, getFillColor: [255, 140, 0, 40], getLineColor: [255, 140, 0, 220],
      lineWidthMinPixels: 1, pickable: true, onClick,
    }),
    new GeoJsonLayer({
      id: "routes", visible: visibility.routes, data: routes,
      stroked: true, getLineColor: [0, 200, 120, 220], lineWidthMinPixels: 2,
      pointType: "circle", getPointRadius: 8, pointRadiusUnits: "pixels",
      pickable: true, onClick,
    }),
    new S2Layer({
      id: "s2", visible: visibility.s2, data: s2CellIds,
      getS2Token: (d: string) => d, filled: false, stroked: true,
      getLineColor: [255, 0, 0, 160], lineWidthMinPixels: 1,
    }),
    ...markerLayers,
  ];
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `bun run test -- layers`
Expected: PASS (2 tests). (Layers construct without a WebGL context; we assert `.id`/`.props` only.)

- [ ] **Step 5: Commit**

```bash
git add apps/web/src/map/lib/layers.ts apps/web/src/map/lib/layers.test.ts
git commit -m "feat(web/map): pure deck layer factory (markers/geofences/routes/s2)"
```

---

### Task 10: Deck canvas (`deck-canvas.tsx`)

**Files:**
- Create: `apps/web/src/map/deck-canvas.tsx`
- Create: `apps/web/src/map/deck-canvas.browser.test.tsx`

**Interfaces:**
- Consumes: all stores (Tasks 2–4), `rasterStyle` (Task 6), `buildLayers` (Task 9), data hooks (Tasks 7–8), `DEFAULT_TILE_URL` from `@/lib/constants`.
- Produces: `<DeckCanvas />` — renders the MapLibre `<Map>` (from `react-map-gl/maplibre`) with a `DeckGLOverlay` child. Owns `onMove` → `setLive`. Registers the throttled-settle subscription. Subscribes (render): `useMapUIStore` layer visibility + s2Level, `useMapSettingsStore` tileServerId + markerRadius. Reads marker/geo/s2 data via the Task 7–8 hooks keyed on `settledBounds`.

- [ ] **Step 1: Write the failing browser smoke test**

`apps/web/src/map/deck-canvas.browser.test.tsx`:

```tsx
import { render } from "vitest-browser-react";
import { expect, test, vi } from "vitest";

// Mock the WebGL-bound libs so the smoke test runs headless.
vi.mock("react-map-gl/maplibre", () => ({
  Map: ({ children, onMove }: any) => (
    <div data-testid="maplibre" onClick={() => onMove?.({ viewState: { longitude: 1, latitude: 2, zoom: 5, pitch: 0, bearing: 0 } })}>
      {children}
    </div>
  ),
  useControl: () => ({}),
}));
vi.mock("@/map/data/use-markers", () => ({ useMarkers: () => ({ data: [] }) }));
vi.mock("@/map/data/use-geo-features", () => ({ useGeoFeatures: () => ({ data: { type: "FeatureCollection", features: [] } }) }));
vi.mock("@/map/data/use-s2-cells", () => ({ useS2Cells: () => ({ data: [] }) }));

import { DeckCanvas } from "@/map/deck-canvas";

test("DeckCanvas mounts the MapLibre root without throwing", async () => {
  const screen = render(<DeckCanvas />);
  await expect.element(screen.getByTestId("maplibre")).toBeInTheDocument();
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `bun run test:browser -- deck-canvas`
Expected: FAIL — `DeckCanvas` not found.

- [ ] **Step 3: Implement `deck-canvas.tsx`**

```tsx
import { useEffect, useMemo } from "react";
import { Map as MapLibre, useControl } from "react-map-gl/maplibre";
import { MapboxOverlay } from "@deck.gl/mapbox";
import "maplibre-gl/dist/maplibre-gl.css";
import { DEFAULT_TILE_URL } from "@/lib/constants";
import { rasterStyle } from "@/map/lib/map-style";
import { buildLayers } from "@/map/lib/layers";
import { useMapViewStore } from "@/map/stores/map-view-store";
import { useMapUIStore } from "@/map/stores/map-ui-store";
import { useMapSettingsStore } from "@/map/stores/map-settings-store";
import { useMarkers } from "@/map/data/use-markers";
import { useGeoFeatures } from "@/map/data/use-geo-features";
import { useS2Cells } from "@/map/data/use-s2-cells";
import type { Bounds } from "@/map/stores/types";

function DeckOverlay({ layers }: { layers: ReturnType<typeof buildLayers> }) {
  const overlay = useControl(() => new MapboxOverlay({ interleaved: false, layers }));
  (overlay as MapboxOverlay).setProps({ layers });
  return null;
}

export function DeckCanvas() {
  // Render-subscriptions: narrow primitives only (S3).
  const visibility = useMapUIStore((s) => s.layerVisibility);
  const s2Level = useMapUIStore((s) => s.s2Level);
  const setSelection = useMapUIStore((s) => s.setSelection);
  const markerRadius = useMapSettingsStore((s) => s.markerRadius);
  const tileServerId = useMapSettingsStore((s) => s.tileServerId); // (Phase 1: maps to DEFAULT_TILE_URL)

  // settledBounds drives the fetches; subscribing to it re-renders ~5×/sec, not per frame.
  const bounds = useMapViewStore((s) => s.settledBounds);

  const gyms = useMarkers("gym", bounds, 0, visibility.gyms);
  const stops = useMarkers("pokestop", bounds, 0, visibility.pokestops);
  const spawns = useMarkers("spawnpoint", bounds, 0, visibility.spawnpoints);
  const stations = useMarkers("station", bounds, 0, visibility.stations);
  const geofences = useGeoFeatures("geofences", visibility.geofences);
  const routes = useGeoFeatures("routes", visibility.routes);
  const s2 = useS2Cells(s2Level, bounds, visibility.s2);

  const layers = useMemo(
    () =>
      buildLayers({
        visibility,
        markerSets: [
          { id: "gyms", points: gyms.data ?? [], color: [230, 80, 80] },
          { id: "pokestops", points: stops.data ?? [], color: [0, 120, 255] },
          { id: "spawnpoints", points: spawns.data ?? [], color: [240, 180, 0] },
          { id: "stations", points: stations.data ?? [], color: [150, 80, 220] },
        ],
        geofences: geofences.data ?? { type: "FeatureCollection", features: [] },
        routes: routes.data ?? { type: "FeatureCollection", features: [] },
        s2CellIds: s2.data ?? [],
        markerRadius,
        onClick: (info) => {
          const id = info.layer?.id ?? "";
          if (id.startsWith("markers-")) setSelection({ kind: "marker", id: String(info.index) });
          else if (id === "geofences") setSelection({ kind: "geofence", id: String(info.index) });
          else if (id === "routes") setSelection({ kind: "route", id: String(info.index) });
        },
      }),
    [visibility, gyms.data, stops.data, spawns.data, stations.data, geofences.data, routes.data, s2.data, markerRadius, setSelection],
  );

  const mapStyle = useMemo(() => rasterStyle(DEFAULT_TILE_URL), [tileServerId]);

  return (
    <MapLibre
      initialViewState={{ longitude: 0, latitude: 0, zoom: 2 }}
      mapStyle={mapStyle}
      onMove={(e: { viewState: { longitude: number; latitude: number; zoom: number; pitch: number; bearing: number } }) => {
        const vs = e.viewState;
        // Transient hot-path write: no component subscribes to liveViewState via a hook.
        const b = boundsFromViewState(vs);
        useMapViewStore.getState().setLive(vs, b);
      }}
      style={{ width: "100%", height: "100%" }}
    >
      <DeckOverlay layers={layers} />
    </MapLibre>
  );
}

/** Approximate bounds from a viewState when the map instance isn't queried directly.
 *  Phase 1 uses the map's own getBounds via onMove event target where available;
 *  this fallback keeps the fetch keyed on a stable bbox. */
function boundsFromViewState(vs: { longitude: number; latitude: number; zoom: number }): Bounds {
  const span = 360 / 2 ** vs.zoom;
  return [vs.longitude - span, vs.latitude - span / 2, vs.longitude + span, vs.latitude + span / 2];
}
```

> **Implementer note:** prefer the real viewport bounds from `e.target.getBounds()` (MapLibre `LngLatBounds`) over `boundsFromViewState` when wiring against the live library — convert to `[minLng,minLat,maxLng,maxLat]`. The fallback exists only so the headless test and the type-check pass without a live GL context. Replace it during Step 5 verification and keep the smoke test green.

- [ ] **Step 4: Run the smoke test to verify it passes**

Run: `bun run test:browser -- deck-canvas`
Expected: PASS (1 test). Run FOREGROUND.

- [ ] **Step 5: Live integration verification**

Start the dev server (`bun run dev`, port 5273) with the koji server running; load `/map`. Confirm: base tiles render, panning fires `onMove` (check React DevTools that `<DeckCanvas>` does NOT re-render every frame — only on settle), and toggling a layer via the store shows/hides it. Swap `boundsFromViewState` for `e.target.getBounds()`. (Per the Claude Preview `document.hidden` trap, verify pan/zoom in a real visible tab if anything seems frozen.)

- [ ] **Step 6: Commit**

```bash
git add apps/web/src/map/deck-canvas.tsx apps/web/src/map/deck-canvas.browser.test.tsx
git commit -m "feat(web/map): deck.gl canvas on MapLibre with transient camera"
```

---

### Task 11: Overlay panels

**Files:**
- Create: `apps/web/src/map/panels/layer-drawer.tsx`
- Create: `apps/web/src/map/panels/coordinate-readout.tsx`
- Create: `apps/web/src/map/panels/tile-server-select.tsx`
- Create: `apps/web/src/map/panels/selection-popup.tsx`
- Create: `apps/web/src/map/panels/layer-drawer.browser.test.tsx`
- Create: `apps/web/src/map/panels/coordinate-readout.browser.test.tsx`

**Interfaces:**
- Consumes: `useMapUIStore`, `useMapViewStore`, `useMapSettingsStore`.
- Produces: four panel components. **Each leaf subscribes to its own primitive (S3); no panel receives a store slice as a prop.** `<LayerDrawer>` renders one `<LayerToggle id>` per `LayerId`; each `<LayerToggle>` selects only its own boolean.

- [ ] **Step 1: Write failing browser tests**

`apps/web/src/map/panels/layer-drawer.browser.test.tsx`:

```tsx
import { render } from "vitest-browser-react";
import { beforeEach, expect, test } from "vitest";
import { LayerDrawer } from "@/map/panels/layer-drawer";
import { useMapUIStore } from "@/map/stores/map-ui-store";

beforeEach(() => useMapUIStore.setState(useMapUIStore.getInitialState()));

test("clicking a layer toggle flips that layer in the store", async () => {
  const screen = render(<LayerDrawer />);
  expect(useMapUIStore.getState().layerVisibility.gyms).toBe(false);
  await screen.getByRole("switch", { name: /gyms/i }).click();
  expect(useMapUIStore.getState().layerVisibility.gyms).toBe(true);
});
```

`apps/web/src/map/panels/coordinate-readout.browser.test.tsx`:

```tsx
import { render } from "vitest-browser-react";
import { beforeEach, expect, test } from "vitest";
import { CoordinateReadout } from "@/map/panels/coordinate-readout";
import { useMapViewStore } from "@/map/stores/map-view-store";

beforeEach(() =>
  useMapViewStore.setState({
    settledViewState: { longitude: -122.33, latitude: 47.6, zoom: 12, pitch: 0, bearing: 0 },
  }),
);

test("readout shows the settled lng/lat/zoom", async () => {
  const screen = render(<CoordinateReadout />);
  await expect.element(screen.getByText(/-122.33/)).toBeInTheDocument();
  await expect.element(screen.getByText(/47.6/)).toBeInTheDocument();
});
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `bun run test:browser -- panels`
Expected: FAIL — modules not found.

- [ ] **Step 3: Implement the panels**

`apps/web/src/map/panels/layer-drawer.tsx`:

```tsx
import { Switch } from "@/components/ui/switch";
import { Label } from "@/components/ui/label";
import { useMapUIStore } from "@/map/stores/map-ui-store";
import type { LayerId } from "@/map/stores/types";

const LAYERS: { id: LayerId; label: string }[] = [
  { id: "geofences", label: "Geofences" }, { id: "routes", label: "Routes" },
  { id: "gyms", label: "Gyms" }, { id: "pokestops", label: "Pokestops" },
  { id: "spawnpoints", label: "Spawnpoints" }, { id: "stations", label: "Stations" },
  { id: "s2", label: "S2 cells" },
];

/** Leaf: subscribes ONLY to its own boolean (S3 — render isolation per toggle). */
function LayerToggle({ id, label }: { id: LayerId; label: string }) {
  const checked = useMapUIStore((s) => s.layerVisibility[id]);
  const toggleLayer = useMapUIStore((s) => s.toggleLayer);
  return (
    <div className="flex items-center justify-between gap-3 py-1">
      <Label htmlFor={`layer-${id}`}>{label}</Label>
      <Switch id={`layer-${id}`} aria-label={label} checked={checked} onCheckedChange={() => toggleLayer(id)} />
    </div>
  );
}

export function LayerDrawer() {
  return (
    <div className="absolute top-4 right-4 z-10 w-56 rounded-lg border bg-background/90 p-3 shadow-md backdrop-blur">
      <p className="mb-2 text-sm font-medium">Layers</p>
      {LAYERS.map((l) => <LayerToggle key={l.id} id={l.id} label={l.label} />)}
    </div>
  );
}
```

`apps/web/src/map/panels/coordinate-readout.tsx`:

```tsx
import { useMapViewStore } from "@/map/stores/map-view-store";

/** Leaf: subscribes ONLY to settled primitives (never liveViewState). */
export function CoordinateReadout() {
  const lng = useMapViewStore((s) => s.settledViewState.longitude);
  const lat = useMapViewStore((s) => s.settledViewState.latitude);
  const zoom = useMapViewStore((s) => s.settledViewState.zoom);
  return (
    <div className="absolute bottom-4 left-4 z-10 rounded bg-background/90 px-2 py-1 font-mono text-xs shadow">
      {lng.toFixed(4)}, {lat.toFixed(4)} · z{zoom.toFixed(1)}
    </div>
  );
}
```

`apps/web/src/map/panels/tile-server-select.tsx`:

```tsx
import {
  Select, SelectContent, SelectItem, SelectTrigger, SelectValue,
} from "@/components/ui/select";
import { useMapSettingsStore } from "@/map/stores/map-settings-store";

/** Phase 1: single "default" option (DEFAULT_TILE_URL). Real tile-server list
 *  from the tile_server resource lands when the switch needs >1 source. */
export function TileServerSelect() {
  const tileServerId = useMapSettingsStore((s) => s.tileServerId);
  const setTileServerId = useMapSettingsStore((s) => s.setTileServerId);
  return (
    <div className="absolute top-4 left-4 z-10">
      <Select value={tileServerId} onValueChange={setTileServerId}>
        <SelectTrigger className="w-40 bg-background/90"><SelectValue /></SelectTrigger>
        <SelectContent><SelectItem value="default">Default</SelectItem></SelectContent>
      </Select>
    </div>
  );
}
```

`apps/web/src/map/panels/selection-popup.tsx`:

```tsx
import { Button } from "@/components/ui/button";
import { useMapUIStore } from "@/map/stores/map-ui-store";

/** Leaf: subscribes to selection only. Phase 1 shows kind+id; deep-link to the
 *  resource edit page lands with editing (Phase 2). */
export function SelectionPopup() {
  const selection = useMapUIStore((s) => s.selection);
  const setSelection = useMapUIStore((s) => s.setSelection);
  if (!selection.kind) return null;
  return (
    <div className="absolute bottom-4 right-4 z-10 w-56 rounded-lg border bg-background/95 p-3 shadow-md">
      <p className="text-sm font-medium capitalize">{selection.kind}</p>
      <p className="text-xs text-muted-foreground">#{selection.id}</p>
      <Button size="sm" variant="ghost" className="mt-2" onClick={() => setSelection({ kind: null, id: null })}>
        Close
      </Button>
    </div>
  );
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `bun run test:browser -- panels`
Expected: PASS (2 tests). (Confirm `@/components/ui/{switch,label,select,button}` exist; if a primitive is missing, add it via `bunx shadcn@latest add <name>` and commit it.)

- [ ] **Step 5: Commit**

```bash
git add apps/web/src/map/panels/
git commit -m "feat(web/map): overlay panels (layers, coords, tile-server, selection)"
```

---

### Task 12: Assemble `MapRoute`, wire nav, full Phase-1 verification

**Files:**
- Modify: `apps/web/src/map/map-route.tsx` (replace placeholder)
- Modify: `apps/web/src/App.tsx` (nav link)
- Create: `apps/web/src/map/map-route.browser.test.tsx`

**Interfaces:**
- Consumes: `<DeckCanvas>` (Task 10), all four panels (Task 11).
- Produces: full-bleed `<MapRoute>` composing canvas + panels.

- [ ] **Step 1: Write the failing assembly smoke test**

`apps/web/src/map/map-route.browser.test.tsx`:

```tsx
import { render } from "vitest-browser-react";
import { expect, test, vi } from "vitest";

vi.mock("@/map/deck-canvas", () => ({ DeckCanvas: () => <div data-testid="deck-canvas" /> }));
import { MapRoute } from "@/map/map-route";

test("MapRoute renders the canvas and the layer drawer together", async () => {
  const screen = render(<MapRoute />);
  await expect.element(screen.getByTestId("deck-canvas")).toBeInTheDocument();
  await expect.element(screen.getByText(/Layers/)).toBeInTheDocument();
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `bun run test:browser -- map-route`
Expected: FAIL — `MapRoute` still the placeholder (no canvas/drawer).

- [ ] **Step 3: Implement the full `MapRoute`**

`apps/web/src/map/map-route.tsx` (replace the placeholder):

```tsx
import { DeckCanvas } from "@/map/deck-canvas";
import { LayerDrawer } from "@/map/panels/layer-drawer";
import { CoordinateReadout } from "@/map/panels/coordinate-readout";
import { TileServerSelect } from "@/map/panels/tile-server-select";
import { SelectionPopup } from "@/map/panels/selection-popup";

export function MapRoute() {
  return (
    <div className="relative h-screen w-screen overflow-hidden">
      <DeckCanvas />
      <TileServerSelect />
      <LayerDrawer />
      <CoordinateReadout />
      <SelectionPopup />
    </div>
  );
}
```

- [ ] **Step 4: Run the assembly test to verify it passes**

Run: `bun run test:browser -- map-route`
Expected: PASS (1 test).

- [ ] **Step 5: Add a nav link to `/map`**

The default shadmin `<Menu>` auto-lists Resources only; `/map` is a CustomRoute, so add an explicit link. Read the current layout/menu wiring first:

Run: `grep -rn "Menu\|layout=\|<Layout" apps/web/src/components/admin/admin.tsx apps/web/src/components/admin/*.tsx | head`

Then add a menu item following ra-core's pattern (a custom `<Menu>` with `<Menu.ResourceItems />` + `<Menu.Item to="/map" primaryText="Map" leftIcon={<MapIcon/>} />`) passed to `<Admin layout=...>` or the existing custom layout. If shadmin exposes a layout slot, add the link there; otherwise the minimum viable link is a `<Link to="/map">` in the dashboard header. Keep it to one focused change.

- [ ] **Step 6: Full Phase-1 verification (parallel)**

Run all four in one batch:
- `bun run typecheck` → Expected: 0 errors.
- `bun run test` → Expected: all unit suites pass (stores, coords, map-style, data hooks, layers).
- `bun run test:browser` → Expected: all browser suites pass (deck-canvas, panels, map-route).
- `bun run build` → Expected: clean production build.

- [ ] **Step 7: Live end-to-end verification**

With the koji server running, `bun run dev` and open `/map`: base tiles render; toggling Geofences/Routes shows real shapes; toggling a marker layer fetches by viewport and renders points; coordinate readout updates on settle (not per frame); selection popup appears on click; reload preserves tile-server choice (persisted). Confirm in React DevTools that panning does NOT re-render `<MapRoute>`/`<DeckCanvas>` per frame.

- [ ] **Step 8: Commit**

```bash
git add apps/web/src/map/map-route.tsx apps/web/src/map/map-route.browser.test.tsx apps/web/src/App.tsx
git commit -m "feat(web/map): assemble full-bleed map route + nav link (Phase 1 complete)"
```

---

## Self-Review

**Spec coverage (Phase 1 scope, spec §8):** base map ✓ (T6/T10) · tile-server switch ✓ (T3/T11, single source Phase 1) · viewport marker fetch ✓ (T7/T10) · geofences/routes ✓ (T8/T10) · S2 ✓ (T8/T10) · LayerDrawer ✓ (T11) · CoordinateReadout ✓ (T11) · SelectionPopup ✓ (T11) · persisted settings ✓ (T3) · transient camera (no 60fps re-render) ✓ (T2/T10). Editing, filters, calc, import/export, realtime → correctly deferred to Phase 2/3 (separate plans).

**Zustand discipline check:** every render-time subscription in T10/T11 selects a narrow primitive in the consuming component (S3); `liveViewState` is written transiently and read only via `getState()` (T10) — never a render hook; no slice is prop-drilled; `<LayerToggle>` isolates each toggle. ✓

**Type consistency:** `LayerId`/`MarkerCategory`/`Bounds`/`ViewState`/`Selection` defined once (T2) and consumed unchanged in T3–T11. `buildLayers` input matches what T10 passes. `apiV2Fetch` signature (T1) matches all data-hook call sites (T7/T8). `setLive(v, bounds)` (T2) matches the T10 call. ✓

**Known soft spots (verify steps included, not placeholders):** exact `/golbat-data` body field (`bbox` vs `area`) — T7 Step 5 greps the Rust; S2 response key (`id` vs `token`) — T8 Step 5 greps the Rust; real viewport bounds vs `boundsFromViewState` fallback — T10 Step 5 swaps to `getBounds()`; envelope-vs-raw response shape — handled defensively in every hook. The nav-menu mechanism — T12 Step 5 reads the layout first.
