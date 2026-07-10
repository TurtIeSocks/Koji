# Per-Entity Maps Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the monolithic overworld `/map` with focused, reusable, record-driven deck.gl map components (mirroring shadmin's Leaflet field/input contract) embedded per-geofence and per-route, with the route edit page as a calc workbench bound to its parent fence.

**Architecture:** A new `apps/web/src/components/deck/` suite, sibling to `components/leaflet/`. Bottom-up: `<DeckMap>` (store-free base, transient per-instance camera) → `<DeckGeoJsonField>` (read-only, reads `record[source]`) → `useDeckEditRHF` + `<DeckGeoJsonInput>` (RHF-controlled editing) → `<GeofenceMap>` / `<RouteMap>` workbenches. The existing `src/map/lib/*` pure helpers are reused. Calc state moves from global zustand to a per-instance headless `useCalc()` hook.

**Tech Stack:** React 19, ra-core (`shadmin-core`), react-hook-form, `@deck.gl/react` + `@deck.gl/layers` + `@deck.gl/core`, `react-map-gl/maplibre` + `maplibre-gl`, `@deck.gl-community/editable-layers`, `@turf/bbox`, `@tanstack/react-query`, vitest (jsdom unit + Playwright browser).

## Global Constraints

- **No backend / migration / API changes.** Fence bbox is computed client-side; calc endpoints already exist.
- **New reusable components live in `apps/web/src/components/deck/`**, NOT `src/map/`. Mirror the Leaflet field/input prop contract (`BaseFieldProps` / `BaseInputProps` in `components/leaflet/types.ts`).
- **Coord discipline:** golbat markers arrive `[lat,lon]` → transpose to `[lng,lat]` for deck (`coords.fromKojiLatLon` / `packMarkers`); geofence/route GeoJSON is already `[lng,lat]`. Marker bbox wire is camelCase (`minLat`…); S2 `BoundsArg` is snake_case (`min_lat`…).
- **deck WebGL renders offscreen-black in Claude Preview.** Verify by DOM geometry + layer props + network, never `preview_screenshot`.
- **Camera stays transient:** never subscribe a render to live viewState; write it via callback only.
- **TDD, frequent commits.** Browser tests for React components (`*.browser.test.tsx`), jsdom units for pure logic (`*.test.ts`). Run `bun run typecheck`, `bun run test`, `bun run test:browser` from `apps/web/`.
- **Commit message convention:** conventional commits, scope `web`, end with the Co-Authored-By trailer.

---

## File Structure

**Create (`apps/web/src/components/deck/`):**
- `bounds.ts` — `boundsToViewState`, `geometryBounds` (turf bbox wrappers).
- `bounds.test.ts`
- `deck-map.tsx` — `<DeckMap>` primitive.
- `deck-map.browser.test.tsx`
- `deck-geojson-field.tsx` — `<DeckGeoJsonField>` read-only.
- `deck-geojson-field.browser.test.tsx`
- `use-deck-edit-rhf.ts` — RHF-controlled edit hook.
- `use-deck-edit-rhf.test.ts`
- `deck-geojson-input.tsx` — `<DeckGeoJsonInput>` editable.
- `deck-geojson-input.browser.test.tsx`
- `geofence-map.tsx` — `<GeofenceMap>` workbench.
- `geofence-map.browser.test.tsx`
- `use-calc.ts` — headless per-instance calc hook.
- `use-calc.test.ts`
- `calc-controls.tsx` — presentational calc panel.
- `calc-controls.browser.test.tsx`
- `route-map.tsx` — `<RouteMap>` calc workbench.
- `route-map.browser.test.tsx`
- `map-index.tsx` — read-only overworld index.
- `map-index.browser.test.tsx`
- `index.ts` — barrel exports.
- `route-mode.ts` — category→route-mode mapping + `route-mode.test.ts`.

**Modify:**
- `resources/geofence/geofence-show.tsx` — swap Leaflet field → `<DeckGeoJsonField>`.
- `resources/geofence/geofence-edit.tsx` — mount `<GeofenceMap>`.
- `resources/geofence/geofence-show.tsx` — add routes `ReferenceManyField` + "New route".
- `resources/route/route-show.tsx` — swap Leaflet field → `<DeckGeoJsonField variant="route">`.
- `resources/route/route-edit.tsx` — mount `<RouteMap>`.
- `App.tsx` — point `/map` at `<MapIndex>`.

---

## PHASE 1 — DeckMap primitive + read-only field

### Task 1: `bounds.ts` helpers + `<DeckMap>` primitive

**Files:**
- Create: `apps/web/src/components/deck/bounds.ts`, `apps/web/src/components/deck/bounds.test.ts`
- Create: `apps/web/src/components/deck/deck-map.tsx`, `apps/web/src/components/deck/deck-map.browser.test.tsx`

**Interfaces:**
- Consumes: `WebMercatorViewport` from `@deck.gl/core`; `bbox` (default) from `@turf/bbox`; `rasterStyle` from `@/map/lib/map-style`; `DEFAULT_TILE_URL` from `@/lib/constants`; `Bounds` from `@/map/stores/types`.
- Produces:
  - `type Bounds = [number, number, number, number]` (re-export from map/stores/types).
  - `geometryBounds(geom: GeoJSON.GeoJsonObject): Bounds | null`
  - `boundsToViewState(bounds: Bounds, width: number, height: number, padding?: number): { longitude: number; latitude: number; zoom: number }`
  - `<DeckMap>` with props `{ layers: Layer[]; fitBounds?: GeoJSON.GeoJsonObject | Bounds | null; initialViewState?: ViewState; height?: number | string; tileUrl?: string; onViewStateChange?: (vs: ViewState, bounds: Bounds) => void; controller?: object; getCursor?: (s: { isDragging: boolean }) => string; children?: ReactNode }`.

- [ ] **Step 1: Write failing test for bounds helpers**

```ts
// bounds.test.ts
import { describe, expect, it } from "vitest";
import { geometryBounds, boundsToViewState } from "./bounds";

describe("geometryBounds", () => {
  it("returns [minLng,minLat,maxLng,maxLat] for a polygon", () => {
    const poly: GeoJSON.Polygon = {
      type: "Polygon",
      coordinates: [[[10, 20], [12, 20], [12, 22], [10, 22], [10, 20]]],
    };
    expect(geometryBounds(poly)).toEqual([10, 20, 12, 22]);
  });
  it("returns null for empty/invalid geometry", () => {
    expect(geometryBounds({ type: "GeometryCollection", geometries: [] })).toBeNull();
  });
});

describe("boundsToViewState", () => {
  it("centers on the bounds midpoint", () => {
    const vs = boundsToViewState([10, 20, 12, 22], 800, 600);
    expect(vs.longitude).toBeCloseTo(11, 1);
    expect(vs.latitude).toBeCloseTo(21, 1);
    expect(vs.zoom).toBeGreaterThan(0);
  });
});
```

- [ ] **Step 2: Run to verify it fails**

Run: `cd apps/web && bun run test src/components/deck/bounds.test.ts`
Expected: FAIL — module not found.

- [ ] **Step 3: Implement `bounds.ts`**

```ts
import bbox from "@turf/bbox";
import { WebMercatorViewport } from "@deck.gl/core";
import type { Bounds } from "@/map/stores/types";

export type { Bounds };

/** Turf bbox → deck Bounds order [minLng,minLat,maxLng,maxLat]. Null when the
 *  geometry has no finite extent (empty collection, no coords). */
export function geometryBounds(geom: GeoJSON.GeoJsonObject): Bounds | null {
  try {
    const b = bbox(geom as GeoJSON.AllGeoJSON);
    if (b.some((n) => !Number.isFinite(n))) return null;
    return [b[0], b[1], b[2], b[3]];
  } catch {
    return null;
  }
}

/** Fit a viewport to bounds. deck has no fitBounds prop, so derive the
 *  initialViewState from a WebMercatorViewport. */
export function boundsToViewState(
  bounds: Bounds,
  width: number,
  height: number,
  padding = 24,
): { longitude: number; latitude: number; zoom: number } {
  const safeW = Math.max(width, 1);
  const safeH = Math.max(height, 1);
  const vp = new WebMercatorViewport({ width: safeW, height: safeH });
  try {
    const { longitude, latitude, zoom } = vp.fitBounds(
      [[bounds[0], bounds[1]], [bounds[2], bounds[3]]],
      { padding: Math.min(padding, Math.floor(Math.min(safeW, safeH) / 2 - 1)) },
    );
    return { longitude, latitude, zoom };
  } catch {
    return { longitude: (bounds[0] + bounds[2]) / 2, latitude: (bounds[1] + bounds[3]) / 2, zoom: 10 };
  }
}
```

- [ ] **Step 4: Run bounds test → PASS**

Run: `cd apps/web && bun run test src/components/deck/bounds.test.ts`
Expected: PASS.

- [ ] **Step 5: Write failing browser test for `<DeckMap>`**

```tsx
// deck-map.browser.test.tsx
import { describe, expect, it } from "vitest";
import { render } from "vitest-browser-react";
import { DeckMap } from "./deck-map";

describe("DeckMap", () => {
  it("mounts the deck + maplibre root without throwing", async () => {
    const screen = render(<DeckMap layers={[]} height={300} />);
    await expect.element(screen.getByTestId("deck-map")).toBeInTheDocument();
  });
});
```

- [ ] **Step 6: Run → FAIL** (`cd apps/web && bun run test:browser src/components/deck/deck-map.browser.test.tsx`) — module not found.

- [ ] **Step 7: Implement `deck-map.tsx`**

Modeled on `src/map/deck-canvas.tsx` (deck as interaction root, MapLibre child) but store-free. Compute `initialViewState` from `fitBounds` once on mount.

```tsx
import { type ReactNode, useMemo, useState } from "react";
import DeckGL from "@deck.gl/react";
import { Map as MapLibre } from "react-map-gl/maplibre";
import { WebMercatorViewport } from "@deck.gl/core";
import type { Layer } from "@deck.gl/core";
import "maplibre-gl/dist/maplibre-gl.css";
import { DEFAULT_TILE_URL } from "@/lib/constants";
import { rasterStyle } from "@/map/lib/map-style";
import type { Bounds } from "@/map/stores/types";
import { geometryBounds, boundsToViewState } from "./bounds";

interface ViewState { longitude: number; latitude: number; zoom: number; pitch: number; bearing: number; }

const DEFAULT_VS: ViewState = { longitude: 0, latitude: 0, zoom: 2, pitch: 0, bearing: 0 };

function isBounds(v: unknown): v is Bounds {
  return Array.isArray(v) && v.length === 4 && v.every((n) => typeof n === "number");
}

export interface DeckMapProps {
  layers: Layer[];
  fitBounds?: GeoJSON.GeoJsonObject | Bounds | null;
  initialViewState?: Partial<ViewState>;
  height?: number | string;
  tileUrl?: string;
  onViewStateChange?: (vs: ViewState, bounds: Bounds) => void;
  controller?: object | boolean;
  getCursor?: (s: { isDragging: boolean }) => string;
  children?: ReactNode;
}

export function DeckMap({
  layers, fitBounds, initialViewState, height = 400,
  tileUrl = DEFAULT_TILE_URL, onViewStateChange, controller = true, getCursor, children,
}: DeckMapProps) {
  // Fit once on mount — a stable initial camera. Live camera stays transient.
  const [initial] = useState<ViewState>(() => {
    if (initialViewState) return { ...DEFAULT_VS, ...initialViewState };
    const b = fitBounds == null ? null : isBounds(fitBounds) ? fitBounds : geometryBounds(fitBounds);
    if (!b) return DEFAULT_VS;
    const w = typeof window !== "undefined" ? window.innerWidth || 800 : 800;
    const h = typeof window !== "undefined" ? window.innerHeight || 600 : 600;
    return { ...DEFAULT_VS, ...boundsToViewState(b, w, h) };
  });
  const mapStyle = useMemo(() => rasterStyle(tileUrl), [tileUrl]);

  return (
    <div data-testid="deck-map" className="relative overflow-hidden rounded-md border" style={{ height, width: "100%" }}>
      <DeckGL
        initialViewState={initial}
        controller={controller}
        layers={layers}
        getCursor={getCursor}
        onViewStateChange={(p) => {
          const vs = p.viewState as unknown as ViewState;
          if (!onViewStateChange) return;
          const w = typeof window !== "undefined" ? window.innerWidth || 800 : 800;
          const h = typeof window !== "undefined" ? window.innerHeight || 600 : 600;
          let bounds: Bounds;
          try {
            const [wst, s, e, n] = new WebMercatorViewport({ ...vs, width: w, height: h }).getBounds();
            bounds = [wst, s, e, n];
          } catch {
            const span = 360 / 2 ** vs.zoom;
            bounds = [vs.longitude - span, vs.latitude - span / 2, vs.longitude + span, vs.latitude + span / 2];
          }
          onViewStateChange(vs, bounds);
        }}
      >
        <MapLibre mapStyle={mapStyle} reuseMaps />
      </DeckGL>
      {children}
    </div>
  );
}
```

- [ ] **Step 8: Run browser test → PASS**

- [ ] **Step 9: Commit**

```bash
git add apps/web/src/components/deck/bounds.ts apps/web/src/components/deck/bounds.test.ts \
  apps/web/src/components/deck/deck-map.tsx apps/web/src/components/deck/deck-map.browser.test.tsx
git commit -m "feat(web): DeckMap primitive + bounds helpers (store-free reusable base)"
```

---

### Task 2: `<DeckGeoJsonField>` read-only field

**Files:**
- Create: `apps/web/src/components/deck/deck-geojson-field.tsx`, `.browser.test.tsx`

**Interfaces:**
- Consumes: `useRecordContext` from `shadmin-core`; `GeoJsonLayer`, `LineLayer` from `@deck.gl/layers`; `routeCoords`, `routeSegments`, `segmentColors` from `@/map/lib/calc-overlay`; `<DeckMap>` + `Bounds` from Task 1.
- Produces: `<DeckGeoJsonField>` props `{ source: string; height?: number|string; tileUrl?: string; fitBounds?: boolean; emptyText?: ReactNode; variant?: "geometry" | "route"; fillColor?: [number,number,number,number]; lineColor?: [number,number,number,number] }`.

- [ ] **Step 1: Write failing browser test**

```tsx
// deck-geojson-field.browser.test.tsx
import { describe, expect, it } from "vitest";
import { render } from "vitest-browser-react";
import { RecordContextProvider } from "shadmin-core";
import { DeckGeoJsonField } from "./deck-geojson-field";

const poly: GeoJSON.Polygon = { type: "Polygon", coordinates: [[[0,0],[1,0],[1,1],[0,1],[0,0]]] };

describe("DeckGeoJsonField", () => {
  it("renders the map when the record has geometry", async () => {
    const screen = render(
      <RecordContextProvider value={{ id: 1, geometry: poly }}>
        <DeckGeoJsonField source="geometry" />
      </RecordContextProvider>,
    );
    await expect.element(screen.getByTestId("deck-map")).toBeInTheDocument();
  });
  it("shows empty text when geometry is missing", async () => {
    const screen = render(
      <RecordContextProvider value={{ id: 1 }}>
        <DeckGeoJsonField source="geometry" emptyText="No geometry" />
      </RecordContextProvider>,
    );
    await expect.element(screen.getByText("No geometry")).toBeVisible();
  });
});
```

- [ ] **Step 2: Run → FAIL.**

- [ ] **Step 3: Implement `deck-geojson-field.tsx`**

```tsx
import { type ReactNode, useMemo } from "react";
import { useRecordContext } from "shadmin-core";
import { GeoJsonLayer, LineLayer } from "@deck.gl/layers";
import type { Layer } from "@deck.gl/core";
import { routeCoords, routeSegments, segmentColors, type RouteSegment } from "@/map/lib/calc-overlay";
import { DeckMap } from "./deck-map";

export interface DeckGeoJsonFieldProps {
  source: string;
  height?: number | string;
  tileUrl?: string;
  fitBounds?: boolean;
  emptyText?: ReactNode;
  variant?: "geometry" | "route";
  fillColor?: [number, number, number, number];
  lineColor?: [number, number, number, number];
}

const DEFAULT_FILL: [number, number, number, number] = [255, 140, 0, 40];
const DEFAULT_LINE: [number, number, number, number] = [255, 140, 0, 220];

export function DeckGeoJsonField({
  source, height = 400, tileUrl, fitBounds = true, emptyText = "No geometry available",
  variant = "geometry", fillColor = DEFAULT_FILL, lineColor = DEFAULT_LINE,
}: DeckGeoJsonFieldProps) {
  const record = useRecordContext();
  const geom = record?.[source] as GeoJSON.GeoJsonObject | null | undefined;

  const layers = useMemo<Layer[]>(() => {
    if (!geom) return [];
    const base = new GeoJsonLayer({
      id: `${source}-geo`, data: geom as GeoJSON.Feature,
      filled: true, getFillColor: fillColor, stroked: true, getLineColor: lineColor,
      lineWidthMinPixels: 2, pointType: "circle", getPointRadius: 5, pointRadiusUnits: "pixels",
    });
    if (variant !== "route") return [base];
    // Route variant: connect the ordered points, colored green→red by leg length.
    const fc: GeoJSON.FeatureCollection = geom.type === "FeatureCollection"
      ? (geom as GeoJSON.FeatureCollection)
      : { type: "FeatureCollection", features: [{ type: "Feature", geometry: geom as GeoJSON.Geometry, properties: {} }] };
    const segs = routeSegments(routeCoords(fc));
    if (segs.length === 0) return [base];
    const colors = segmentColors(segs);
    const path = new LineLayer<RouteSegment>({
      id: `${source}-route`, data: segs,
      getSourcePosition: (s) => s.source, getTargetPosition: (s) => s.target,
      getColor: (_s, info) => colors[info.index], getWidth: 3, widthUnits: "pixels", widthMinPixels: 2,
    });
    return [path, base];
  }, [geom, source, variant, fillColor, lineColor]);

  if (!geom) {
    return (
      <div style={{ height, width: "100%" }}
        className="flex items-center justify-center rounded-md border bg-muted/30 text-sm text-muted-foreground"
        data-slot="deck-field-empty">
        {emptyText}
      </div>
    );
  }
  return <DeckMap layers={layers} fitBounds={fitBounds ? geom : null} height={height} tileUrl={tileUrl} controller={{ doubleClickZoom: true }} />;
}
```

- [ ] **Step 4: Run → PASS.**

- [ ] **Step 5: Commit**

```bash
git add apps/web/src/components/deck/deck-geojson-field.tsx apps/web/src/components/deck/deck-geojson-field.browser.test.tsx
git commit -m "feat(web): DeckGeoJsonField — read-only record-driven deck field (drop-in for Leaflet GeoJsonField/MultiPointField)"
```

---

### Task 3: Swap show pages to `<DeckGeoJsonField>`

**Files:**
- Modify: `apps/web/src/resources/geofence/geofence-show.tsx`, `apps/web/src/resources/route/route-show.tsx`
- Modify tests: `apps/web/src/resources/geofence/geofence-show.browser.test.tsx` (if it asserts the Leaflet field), `apps/web/src/resources/route/route-show.tsx` test if present.

**Interfaces:**
- Consumes: `DeckGeoJsonField` from `@/components/deck`.

- [ ] **Step 1: Update the barrel** — create/extend `apps/web/src/components/deck/index.ts`:

```ts
export * from "./deck-map";
export * from "./deck-geojson-field";
```

- [ ] **Step 2: Modify `geofence-show.tsx`** — replace the Leaflet field:

Remove `import { GeoJsonField } from "@/components/leaflet";` usage of `<GeoJsonField source="geometry" tileUrl={DEFAULT_TILE_URL} height={400} />` and replace with:

```tsx
import { DeckGeoJsonField } from "@/components/deck";
// …
<DeckGeoJsonField source="geometry" height={400} />
```

- [ ] **Step 3: Modify `route-show.tsx`** — replace `<MultiPointField … />` with:

```tsx
import { DeckGeoJsonField } from "@/components/deck";
// …
<DeckGeoJsonField source="geometry" variant="route" height={400} />
```

- [ ] **Step 4: Run the show-page browser tests + typecheck**

Run: `cd apps/web && bun run typecheck && bun run test:browser src/resources/geofence/geofence-show.browser.test.tsx`
Expected: PASS (update any assertion that referenced Leaflet-specific DOM to `getByTestId("deck-map")`).

- [ ] **Step 5: Commit**

```bash
git add apps/web/src/components/deck/index.ts apps/web/src/resources/geofence/geofence-show.tsx apps/web/src/resources/route/route-show.tsx apps/web/src/resources/geofence/geofence-show.browser.test.tsx
git commit -m "feat(web): geofence-show + route-show render the reusable DeckGeoJsonField"
```

---

## PHASE 2 — Editable input (`useDeckEditRHF` + `<DeckGeoJsonInput>`)

### Task 4: `useDeckEditRHF` hook

**Files:**
- Create: `apps/web/src/components/deck/use-deck-edit-rhf.ts`, `.test.ts`

**Interfaces:**
- Consumes: `useWatch`, `useFormContext` from `react-hook-form`; `shouldCommitEdit`, `explodeSplitFeatures` from `@/map/lib/edit-serialize`; `DrawMode` from `@/map/lib/edit-modes`.
- Produces:
  ```ts
  interface UseDeckEditRHFOptions {
    source: string;
    /** How the form value maps to/from the draft FeatureCollection. */
    toFeature?: (value: unknown) => GeoJSON.Feature | null;   // default: value is a Geometry
    fromFeatures?: (features: GeoJSON.Feature[], prev: unknown) => unknown; // default: first feature's geometry
  }
  interface UseDeckEditRHFReturn {
    draft: GeoJSON.FeatureCollection;
    mode: DrawMode;
    setMode: (m: DrawMode) => void;
    selectedIndexes: number[];
    onEdit: DraftInput["onEdit"];
    onSelect: (indexes: number[]) => void;
  }
  function useDeckEditRHF(opts: UseDeckEditRHFOptions): UseDeckEditRHFReturn
  ```

The hook holds `draft`/`mode`/`selectedIndexes` in local `useState` (per-instance — no global store), hydrates `draft` from the watched form value (with echo-dedup so its own writes don't re-hydrate), and commits geometry back via `form.setValue` on every committed edit.

- [ ] **Step 1: Write failing unit test** (jsdom; wrap in a RHF harness via `renderHook` with `FormProvider`).

```ts
// use-deck-edit-rhf.test.ts
import { describe, expect, it, vi } from "vitest";
import { renderHook, act } from "@testing-library/react";
import { FormProvider, useForm } from "react-hook-form";
import type { ReactNode } from "react";
import { useDeckEditRHF } from "./use-deck-edit-rhf";

const poly: GeoJSON.Polygon = { type: "Polygon", coordinates: [[[0,0],[1,0],[1,1],[0,1],[0,0]]] };

function wrapper(defaultValues: Record<string, unknown>) {
  return ({ children }: { children: ReactNode }) => {
    const form = useForm({ defaultValues });
    return <FormProvider {...form}>{children}</FormProvider>;
  };
}

describe("useDeckEditRHF", () => {
  it("hydrates the draft from the form value", () => {
    const { result } = renderHook(() => useDeckEditRHF({ source: "geometry" }), {
      wrapper: wrapper({ geometry: poly }),
    });
    expect(result.current.draft.features).toHaveLength(1);
    expect(result.current.draft.features[0].geometry).toEqual(poly);
  });

  it("commits an edited geometry back to the form value", () => {
    const captured: unknown[] = [];
    const Probe = () => {
      const hook = useDeckEditRHF({ source: "geometry" });
      // expose setValue effect by reading form after edit — captured via onEdit
      const moved: GeoJSON.FeatureCollection = {
        type: "FeatureCollection",
        features: [{ type: "Feature", geometry: { type: "Polygon", coordinates: [[[0,0],[2,0],[2,2],[0,2],[0,0]]] }, properties: {} }],
      };
      // simulate a committed modify edit
      // (call in an effect-free manner for the test)
      (Probe as any)._edit = () => hook.onEdit({ updatedData: moved, editType: "movePosition" });
      return null;
    };
    // Assert via a spy on setValue instead:
    const spy = vi.fn();
    const { result } = renderHook(() => useDeckEditRHF({ source: "geometry" }), {
      wrapper: ({ children }) => {
        const form = useForm({ defaultValues: { geometry: poly } });
        vi.spyOn(form, "setValue").mockImplementation((...a) => { spy(...a); });
        return <FormProvider {...form}>{children}</FormProvider>;
      },
    });
    act(() => {
      result.current.onEdit({
        updatedData: { type: "FeatureCollection", features: [{ type: "Feature", geometry: { type: "Polygon", coordinates: [[[0,0],[2,0],[2,2],[0,2],[0,0]]] }, properties: {} }] },
        editType: "movePosition",
      });
    });
    expect(spy).toHaveBeenCalledWith("geometry", expect.objectContaining({ type: "Polygon" }), expect.anything());
    void captured;
  });
});
```

- [ ] **Step 2: Run → FAIL.**

- [ ] **Step 3: Implement `use-deck-edit-rhf.ts`**

```ts
import { useCallback, useEffect, useRef, useState } from "react";
import { useFormContext, useWatch } from "react-hook-form";
import { shouldCommitEdit, explodeSplitFeatures } from "@/map/lib/edit-serialize";
import type { DrawMode } from "@/map/lib/edit-modes";

const EMPTY_FC: GeoJSON.FeatureCollection = { type: "FeatureCollection", features: [] };

function defaultToFeature(value: unknown): GeoJSON.Feature | null {
  if (value == null) return null;
  const v = value as GeoJSON.Feature | GeoJSON.Geometry;
  if ((v as GeoJSON.Feature).type === "Feature") return v as GeoJSON.Feature;
  return { type: "Feature", geometry: v as GeoJSON.Geometry, properties: {} };
}
function defaultFromFeatures(features: GeoJSON.Feature[]): unknown {
  return features[0]?.geometry ?? null;
}

export interface UseDeckEditRHFOptions {
  source: string;
  toFeature?: (value: unknown) => GeoJSON.Feature | null;
  fromFeatures?: (features: GeoJSON.Feature[], prev: unknown) => unknown;
}

export interface UseDeckEditRHFReturn {
  draft: GeoJSON.FeatureCollection;
  mode: DrawMode;
  setMode: (m: DrawMode) => void;
  selectedIndexes: number[];
  onEdit: (e: { updatedData: GeoJSON.FeatureCollection; editType?: string; editContext?: { featureIndexes?: number[] } }) => void;
  onSelect: (indexes: number[]) => void;
}

export function useDeckEditRHF({ source, toFeature = defaultToFeature, fromFeatures = defaultFromFeatures }: UseDeckEditRHFOptions): UseDeckEditRHFReturn {
  const form = useFormContext();
  const value = useWatch({ name: source });
  const [draft, setDraft] = useState<GeoJSON.FeatureCollection>(EMPTY_FC);
  const [mode, setMode] = useState<DrawMode>("none");
  const [selectedIndexes, setSelectedIndexes] = useState<number[]>([]);
  const lastWritten = useRef<unknown>(undefined);
  const hydrated = useRef(false);

  // Hydrate from the form value; skip the echo of our own writes.
  useEffect(() => {
    if (hydrated.current && JSON.stringify(value) === JSON.stringify(lastWritten.current)) return;
    const feat = toFeature(value);
    setDraft(feat ? { type: "FeatureCollection", features: [feat] } : EMPTY_FC);
    lastWritten.current = value;
    hydrated.current = true;
  }, [value, toFeature]);

  const commit = useCallback((fc: GeoJSON.FeatureCollection) => {
    setDraft(fc);
    const prev = form.getValues(source);
    const stored = fromFeatures(fc.features, prev);
    lastWritten.current = stored;
    form.setValue(source, stored, { shouldDirty: true });
  }, [form, source, fromFeatures]);

  const onEdit = useCallback((e: { updatedData: GeoJSON.FeatureCollection; editType?: string; editContext?: { featureIndexes?: number[] } }) => {
    if (!shouldCommitEdit(e.editType)) { setDraft(e.updatedData); return; }
    if (e.editType === "split" && e.editContext?.featureIndexes) {
      commit(explodeSplitFeatures(e.updatedData, e.editContext.featureIndexes));
    } else {
      commit(e.updatedData);
    }
  }, [commit]);

  return { draft, mode, setMode, selectedIndexes, onEdit, onSelect: setSelectedIndexes };
}
```

- [ ] **Step 4: Run → PASS.**

- [ ] **Step 5: Commit**

```bash
git add apps/web/src/components/deck/use-deck-edit-rhf.ts apps/web/src/components/deck/use-deck-edit-rhf.test.ts
git commit -m "feat(web): useDeckEditRHF — per-instance RHF-controlled deck editing (deck analog of useGeomanRHF)"
```

---

### Task 5: `<DeckGeoJsonInput>` editable input

**Files:**
- Create: `apps/web/src/components/deck/deck-geojson-input.tsx`, `.browser.test.tsx`

**Interfaces:**
- Consumes: `useDeckEditRHF` (Task 4); `buildBaseLayers`, `buildEditLayer` from `@/map/lib/layers`; `DrawToolbar` pattern — reuse the button set but drive from `mode`/`setMode` props (NOT the global ui-store). Create a local `<DeckDrawToolbar mode setMode selectedCount>` in this file (small, presentational) rather than importing the store-bound `src/map/panels/draw-toolbar.tsx`.
- Produces: `<DeckGeoJsonInput>` props `{ source: string; label?: ReactNode; helperText?: ReactNode; height?: number|string; tileUrl?: string; disabled?: boolean; toFeature?; fromFeatures?; contextLayers?: Layer[] }`.

- [ ] **Step 1: Write failing browser test** (RHF `SimpleForm` harness; assert the map + toolbar render, and that drawing mode toggles).

```tsx
// deck-geojson-input.browser.test.tsx
import { describe, expect, it } from "vitest";
import { render } from "vitest-browser-react";
import { AdminContext } from "@/components/admin";
import { SimpleForm } from "@/components/admin";
import { testDataProvider } from "shadmin-core";
import { DeckGeoJsonInput } from "./deck-geojson-input";

describe("DeckGeoJsonInput", () => {
  it("renders the edit map + draw toolbar inside a form", async () => {
    const screen = render(
      <AdminContext dataProvider={testDataProvider()}>
        <SimpleForm onSubmit={() => {}}>
          <DeckGeoJsonInput source="geometry" label="Geometry" />
        </SimpleForm>
      </AdminContext>,
    );
    await expect.element(screen.getByTestId("deck-map")).toBeInTheDocument();
    await expect.element(screen.getByRole("button", { name: /polygon/i })).toBeInTheDocument();
  });
});
```

- [ ] **Step 2: Run → FAIL.**

- [ ] **Step 3: Implement `deck-geojson-input.tsx`**

```tsx
import { type ReactNode, useMemo } from "react";
import type { Layer } from "@deck.gl/core";
import { buildEditLayer } from "@/map/lib/layers";
import type { DrawMode } from "@/map/lib/edit-modes";
import { Button } from "@/components/ui/button";
import { DeckMap } from "./deck-map";
import { geometryBounds } from "./bounds";
import { useDeckEditRHF, type UseDeckEditRHFOptions } from "./use-deck-edit-rhf";

// DrawMode union (from edit-modes.ts) is:
//   "none" | "drawPolygon" | "drawRectangle" | "drawCircle" | "modify"
//   | "transform" | "split" | "cutHole"
// Core geofence-editing set only (split/cutHole need a prior selection — omit for v1).
const DRAW_BUTTONS: { mode: DrawMode; label: string }[] = [
  { mode: "drawPolygon", label: "Polygon" },
  { mode: "drawRectangle", label: "Rectangle" },
  { mode: "modify", label: "Modify" },
  { mode: "transform", label: "Move" },
];

function DeckDrawToolbar({ mode, setMode }: { mode: DrawMode; setMode: (m: DrawMode) => void }) {
  return (
    <div className="absolute top-2 left-2 z-10 flex gap-1 rounded-md bg-background/90 p-1 shadow-md backdrop-blur">
      {DRAW_BUTTONS.map((b) => (
        <Button key={b.mode} size="sm" variant={mode === b.mode ? "default" : "secondary"}
          onClick={() => setMode(mode === b.mode ? "none" : b.mode)}>{b.label}</Button>
      ))}
    </div>
  );
}

export interface DeckGeoJsonInputProps extends UseDeckEditRHFOptions {
  label?: ReactNode;
  helperText?: ReactNode;
  height?: number | string;
  tileUrl?: string;
  disabled?: boolean;
  /** Extra read-only layers drawn under the edit layer (e.g. marker/S2 context). */
  contextLayers?: Layer[];
}

export function DeckGeoJsonInput({ label, helperText, height = 400, tileUrl, disabled, contextLayers, ...editOpts }: DeckGeoJsonInputProps) {
  const { draft, mode, setMode, selectedIndexes, onEdit, onSelect } = useDeckEditRHF(editOpts);

  const editLayers = useMemo<Layer[]>(
    () => buildEditLayer({ mode, features: draft, selectedIndexes, onEdit, onSelect }),
    [mode, draft, selectedIndexes, onEdit, onSelect],
  );
  const layers = useMemo(() => [...(contextLayers ?? []), ...editLayers], [contextLayers, editLayers]);
  const fit = draft.features[0]?.geometry ? geometryBounds(draft.features[0].geometry) : null;

  return (
    <div className="flex flex-col gap-1" data-slot="deck-geojson-input">
      {label ? <span className="text-sm font-medium">{label}</span> : null}
      <div className="relative" style={{ height }}>
        <DeckMap
          layers={layers}
          fitBounds={fit}
          height={height}
          tileUrl={tileUrl}
          controller={{ doubleClickZoom: false }}
          getCursor={({ isDragging }) => (mode !== "none" ? "crosshair" : isDragging ? "grabbing" : "grab")}
        >
          {!disabled ? <DeckDrawToolbar mode={mode} setMode={setMode} /> : null}
        </DeckMap>
      </div>
      {helperText ? <div className="text-xs text-muted-foreground">{helperText}</div> : null}
    </div>
  );
}
```

> Note: confirm `DrawMode` includes `"polygon" | "rectangle" | "modify" | "transform" | "none"` in `edit-modes.ts`; if a label differs, use the exact union member. Read `edit-modes.ts` before implementing.

- [ ] **Step 4: Run → PASS.** Add to barrel `index.ts`: `export * from "./use-deck-edit-rhf"; export * from "./deck-geojson-input";`

- [ ] **Step 5: Commit**

```bash
git add apps/web/src/components/deck/deck-geojson-input.tsx apps/web/src/components/deck/deck-geojson-input.browser.test.tsx apps/web/src/components/deck/index.ts
git commit -m "feat(web): DeckGeoJsonInput — RHF-controlled editable deck field (drop-in for Leaflet GeoJsonInput)"
```

---

### Task 6: Geometry editing in `geofence-edit`

**Files:**
- Modify: `apps/web/src/resources/geofence/geofence-edit.tsx`
- Modify test: `apps/web/src/resources/geofence/geofence-form.browser.test.tsx` (add geometry-input assertion).

- [ ] **Step 1: Add a failing assertion** that geofence-edit renders the deck geometry input (`getByTestId("deck-map")`).

- [ ] **Step 2: Modify `geofence-edit.tsx`** — add the input into the edit form:

```tsx
import { DeckGeoJsonInput } from "@/components/deck";
// inside GeofenceEditFields, after GeofenceFormFields:
<DeckGeoJsonInput source="geometry" label="Geometry" height={400} />
```

- [ ] **Step 3: Run the geofence edit/form browser test + typecheck → PASS.**

- [ ] **Step 4: Commit**

```bash
git add apps/web/src/resources/geofence/geofence-edit.tsx apps/web/src/resources/geofence/geofence-form.browser.test.tsx
git commit -m "feat(web): geofence-edit gains map geometry editing via DeckGeoJsonInput"
```

---

## PHASE 3 — Geofence workbench

### Task 7: `<GeofenceMap>` — editing + fence-scoped marker/S2 context

**Files:**
- Create: `apps/web/src/components/deck/geofence-map.tsx`, `.browser.test.tsx`

**Interfaces:**
- Consumes: `DeckGeoJsonInput` (Task 5); `geometryBounds` (Task 1); `useMarkers` from `@/map/data/use-markers`; `useS2Cells` from `@/map/data/use-s2-cells`; `buildBaseLayers` from `@/map/lib/layers`; `useRecordContext`, `useWatch`.
- Produces: `<GeofenceMap>` (no props — reads record + form). Builds `contextLayers` (markers/S2 within the fence bbox) and passes them to `<DeckGeoJsonInput source="geometry">`. Local `useState` layer toggles.

- [ ] **Step 1: Write failing browser test** — with a record holding a polygon, `useMarkers` is called with the fence bbox; the map renders. (Stub `useMarkers`/`useS2Cells` via `vi.mock` to assert the bbox argument.)

```tsx
// geofence-map.browser.test.tsx  (essential assertions)
import { describe, expect, it, vi } from "vitest";
vi.mock("@/map/data/use-markers", () => ({ useMarkers: vi.fn(() => ({ data: [] })) }));
vi.mock("@/map/data/use-s2-cells", () => ({ useS2Cells: vi.fn(() => ({ data: [] })) }));
import { render } from "vitest-browser-react";
import { AdminContext, SimpleForm } from "@/components/admin";
import { RecordContextProvider, testDataProvider } from "shadmin-core";
import { useMarkers } from "@/map/data/use-markers";
import { GeofenceMap } from "./geofence-map";

const poly: GeoJSON.Polygon = { type: "Polygon", coordinates: [[[0,0],[2,0],[2,2],[0,2],[0,0]]] };

describe("GeofenceMap", () => {
  it("scopes markers to the fence bbox", async () => {
    const screen = render(
      <AdminContext dataProvider={testDataProvider()}>
        <RecordContextProvider value={{ id: 1, geometry: poly }}>
          <SimpleForm onSubmit={() => {}} record={{ id: 1, geometry: poly }}>
            <GeofenceMap />
          </SimpleForm>
        </RecordContextProvider>
      </AdminContext>,
    );
    await expect.element(screen.getByTestId("deck-map")).toBeInTheDocument();
    // bbox of poly = [0,0,2,2]
    expect(useMarkers).toHaveBeenCalledWith("gym", [0,0,2,2], expect.anything(), expect.anything());
  });
});
```

- [ ] **Step 2: Run → FAIL.**

- [ ] **Step 3: Implement `geofence-map.tsx`**

```tsx
import { useMemo, useState } from "react";
import { useWatch } from "react-hook-form";
import type { Layer } from "@deck.gl/core";
import { buildBaseLayers } from "@/map/lib/layers";
import { useMarkers } from "@/map/data/use-markers";
import { useS2Cells } from "@/map/data/use-s2-cells";
import type { Bounds } from "@/map/stores/types";
import { Button } from "@/components/ui/button";
import { DeckGeoJsonInput } from "./deck-geojson-input";
import { geometryBounds } from "./bounds";

const WORLD: Bounds = [-180, -85, 180, 85];

export function GeofenceMap() {
  // Live geometry from the form drives the marker/S2 bbox as the fence is edited.
  const geometry = useWatch({ name: "geometry" }) as GeoJSON.Geometry | null | undefined;
  const bbox = useMemo<Bounds>(() => (geometry ? geometryBounds(geometry) ?? WORLD : WORLD), [geometry]);

  const [show, setShow] = useState({ gyms: false, pokestops: false, spawnpoints: false, s2: false });
  const gyms = useMarkers("gym", bbox, 0, show.gyms);
  const stops = useMarkers("pokestop", bbox, 0, show.pokestops);
  const spawns = useMarkers("spawnpoint", bbox, 0, show.spawnpoints);
  const s2 = useS2Cells(15, bbox, show.s2);

  const contextLayers = useMemo<Layer[]>(
    () => buildBaseLayers({
      visibility: { gyms: show.gyms, pokestops: show.pokestops, spawnpoints: show.spawnpoints, stations: false, geofences: false, routes: false, s2: show.s2 } as Record<string, boolean>,
      markerSets: [
        { id: "gyms", points: gyms.data ?? [], color: [230, 80, 80] },
        { id: "pokestops", points: stops.data ?? [], color: [0, 120, 255] },
        { id: "spawnpoints", points: spawns.data ?? [], color: [240, 180, 0] },
      ],
      geofences: { type: "FeatureCollection", features: [] },
      routes: { type: "FeatureCollection", features: [] },
      s2Cells: s2.data ?? [],
      markerRadius: 70,
      onClick: () => {},
      pickable: false,
    }),
    [show, gyms.data, stops.data, spawns.data, s2.data],
  );

  return (
    <div className="flex flex-col gap-2">
      <div className="flex flex-wrap gap-1">
        {(["gyms", "pokestops", "spawnpoints", "s2"] as const).map((k) => (
          <Button key={k} size="sm" variant={show[k] ? "default" : "secondary"}
            onClick={() => setShow((s) => ({ ...s, [k]: !s[k] }))} className="capitalize">{k}</Button>
        ))}
      </div>
      <DeckGeoJsonInput source="geometry" label="Geometry" height={480} contextLayers={contextLayers} />
    </div>
  );
}
```

- [ ] **Step 4: Run → PASS.** Barrel: add `export * from "./geofence-map";`

- [ ] **Step 5: Swap `geofence-edit.tsx`** to use `<GeofenceMap />` instead of the bare `<DeckGeoJsonInput>` from Task 6 (GeofenceMap supersedes it — it embeds the input + context). Update the Task-6 assertion if needed.

- [ ] **Step 6: Commit**

```bash
git add apps/web/src/components/deck/geofence-map.tsx apps/web/src/components/deck/geofence-map.browser.test.tsx apps/web/src/components/deck/index.ts apps/web/src/resources/geofence/geofence-edit.tsx
git commit -m "feat(web): GeofenceMap workbench — polygon editing + fence-scoped marker/S2 context"
```

---

### Task 8: Geofence → routes list + "New route"

**Files:**
- Modify: `apps/web/src/resources/geofence/geofence-show.tsx`
- Modify: `apps/web/src/data-provider.ts` (only if the `geofence_id` filter param needs special-casing — see Step 2).
- Modify test: `apps/web/src/resources/geofence/geofence-show.browser.test.tsx`

**Interfaces:**
- Consumes: `ReferenceManyField`, `DataTable`, `CreateButton` from `@/components/admin`; the getManyReference `target`→filter translation in `data-provider.ts`.

- [ ] **Step 1: Confirm the route list filter param.** Read `crates/koji-service/src/public/v2/routes.rs` for the query arg name. The generic `getManyReference` maps `target="geofence_id"` → `?geofence=`. If the backend expects `geofence_id` or `geofenceid`, add a special case:

```ts
// data-provider.ts getManyReference — replace the _id-strip line with a map:
const TARGET_TO_PARAM: Record<string, string> = { project_id: "project", geofence_id: "geofence_id" };
const filterParam = params.target ? (TARGET_TO_PARAM[params.target] ?? params.target.replace(/_id$/, "")) : undefined;
```

Pick the exact right-hand value from the backend. Add/adjust the doc comment.

- [ ] **Step 2: Write a failing browser test** — geofence-show lists a route scoped to the fence (assert `getManyReference` receives `target="geofence_id"`, `id=<fence>`), mirroring the webhook section test in `project-show.browser.test.tsx`.

- [ ] **Step 3: Add the section to `geofence-show.tsx`**

```tsx
import { ReferenceManyField, DataTable, CreateButton, BooleanField } from "@/components/admin";
import { useRecordContext } from "ra-core";

const NewRouteButton = () => {
  const record = useRecordContext();
  if (!record) return null;
  return <CreateButton resource="route" label="New route" />;
};

// inside the show layout:
<ReferenceManyField reference="route" target="geofence_id" label="Routes">
  <div className="flex flex-col gap-2">
    <div className="flex justify-end"><NewRouteButton /></div>
    <DataTable bulkActionButtons={false}>
      <DataTable.Col source="name" />
      <DataTable.Col source="mode" />
      <DataTable.Col source="points" label="Points" />
    </DataTable>
  </div>
</ReferenceManyField>
```

- [ ] **Step 4: Run browser test + typecheck → PASS.** Verify against the real backend that `?geofence…=<id>` scopes the list (per Global Constraints, do this by DOM/network, not screenshot).

- [ ] **Step 5: Commit**

```bash
git add apps/web/src/resources/geofence/geofence-show.tsx apps/web/src/resources/geofence/geofence-show.browser.test.tsx apps/web/src/data-provider.ts
git commit -m "feat(web): geofence-show lists its routes + New-route button (getManyReference geofence_id)"
```

---

## PHASE 4 — Route calc workbench

### Task 9: Headless `useCalc()` hook

**Files:**
- Create: `apps/web/src/components/deck/use-calc.ts`, `.test.ts`

**Interfaces:**
- Consumes: `submitCalc`, `getJob`, `type JobRecord` from `@/map/data/calc-client`; `buildCalcBody`, `parseCalcResult`, `type CalcParams`, `type CalcInputs`, `type CalcMode` from `@/map/lib/calc-request`; `useSubscribe` from `@/components/realtime`.
- Produces: a per-instance hook (mirrors `map-calc-store` fields + `useCalcJob` behavior, but local state — NO global store):
  ```ts
  interface UseCalcReturn {
    params: CalcParams; setParams: (p: Partial<CalcParams>) => void;
    job: { id: string; status: string; progress: number; phase: string | null } | null;
    result: GeoJSON.FeatureCollection | null; stats: unknown; error: string | null;
    run: (inputs: CalcInputs) => Promise<void>;
    clear: () => void;
  }
  function useCalc(initial?: Partial<CalcParams>): UseCalcReturn
  ```

- [ ] **Step 1: Write failing unit test** (mock `calc-client` + `useSubscribe`): `run()` calls `submitCalc` with the body from `buildCalcBody`, sets `job`; a succeeded `getJob` resolves `result`.

```ts
// use-calc.test.ts (essentials)
import { describe, expect, it, vi, beforeEach } from "vitest";
import { renderHook, act, waitFor } from "@testing-library/react";
vi.mock("@/components/realtime", () => ({ useSubscribe: vi.fn() }));
vi.mock("@/map/data/calc-client", () => ({
  submitCalc: vi.fn(async () => "42"),
  getJob: vi.fn(async () => ({ id: 42, status: "Succeeded", progress: 1, phase: null, result: { data: { type: "FeatureCollection", features: [] }, stats: { total_clusters: 3 } } })),
}));
import { submitCalc } from "@/map/data/calc-client";
import { useCalc } from "./use-calc";

describe("useCalc", () => {
  beforeEach(() => vi.clearAllMocks());
  it("submits a calc and resolves the result", async () => {
    const { result } = renderHook(() => useCalc({ mode: "route", category: "spawnpoint" }));
    await act(async () => { await result.current.run({ area: { type: "FeatureCollection", features: [] } }); });
    expect(submitCalc).toHaveBeenCalled();
    await waitFor(() => expect(result.current.result).not.toBeNull());
  });
});
```

- [ ] **Step 2: Run → FAIL.**

- [ ] **Step 3: Implement `use-calc.ts`** — reducer-free local `useState`; on `run`, `submitCalc(buildCalcBody(params, inputs))` → set job → `getJob` safety-net fetch + `useSubscribe(\`jobs/${id}\`)` for live progress; normalize PascalCase status (`"Succeeded"` → lower); on succeeded, `parseCalcResult` → `result`/`stats`. (Port the terminal-resolution logic from `use-calc-job.ts` into local setters.)

```ts
import { useCallback, useEffect, useRef, useState } from "react";
import { useSubscribe } from "@/components/realtime";
import { submitCalc, getJob } from "@/map/data/calc-client";
import { buildCalcBody, parseCalcResult, type CalcParams, type CalcInputs } from "@/map/lib/calc-request";

interface Job { id: string; status: string; progress: number; phase: string | null; }
const norm = (s: string | undefined) => (s ?? "").toLowerCase();
const DEFAULTS: CalcParams = { mode: "cluster", category: "pokestop", radius: 70, minPoints: 1, clusterMode: null, sortBy: null };

export interface UseCalcReturn {
  params: CalcParams; setParams: (p: Partial<CalcParams>) => void;
  job: Job | null; result: GeoJSON.FeatureCollection | null; stats: unknown; error: string | null;
  run: (inputs: CalcInputs) => Promise<void>; clear: () => void;
}

export function useCalc(initial?: Partial<CalcParams>): UseCalcReturn {
  const [params, setParamsState] = useState<CalcParams>({ ...DEFAULTS, ...initial });
  const [job, setJob] = useState<Job | null>(null);
  const [result, setResult] = useState<GeoJSON.FeatureCollection | null>(null);
  const [stats, setStats] = useState<unknown>(null);
  const [error, setError] = useState<string | null>(null);
  const jobId = job?.id ?? null;
  const resolvedRef = useRef<string | null>(null);

  const setParams = useCallback((p: Partial<CalcParams>) => setParamsState((s) => ({ ...s, ...p })), []);

  const resolveTerminal = useCallback(async (id: string) => {
    if (resolvedRef.current === id) return;
    resolvedRef.current = id;
    try {
      const rec = await getJob(id);
      const st = norm(rec.status);
      if (st === "succeeded") { const r = parseCalcResult(rec); setResult(r.fc); setStats(r.stats); setJob((j) => j && { ...j, status: "succeeded", progress: 1 }); }
      else if (st === "failed") { setError(rec.error ?? "job failed"); setJob((j) => j && { ...j, status: "failed" }); }
    } catch (e) { setError(e instanceof Error ? e.message : "failed to fetch job result"); }
  }, []);

  useSubscribe<{ status?: string; progress?: number; phase?: string | null }>(
    jobId ? `jobs/${jobId}` : "",
    (event) => {
      const p = event.payload; const st = norm(p?.status);
      if (!st || !jobId) return;
      setJob((j) => j && { ...j, status: st, progress: p.progress ?? j.progress, phase: p.phase ?? j.phase });
      if (st === "succeeded" || st === "failed") void resolveTerminal(jobId);
    },
    { enabled: jobId != null },
  );

  // Safety net: fetch once when a job appears (finished-before-subscribe / reload).
  useEffect(() => { if (jobId) void resolveTerminal(jobId); }, [jobId, resolveTerminal]);

  const run = useCallback(async (inputs: CalcInputs) => {
    setError(null); setResult(null); setStats(null); resolvedRef.current = null;
    try {
      const id = await submitCalc(buildCalcBody(params, inputs));
      setJob({ id, status: "queued", progress: 0, phase: null });
    } catch (e) { setError(e instanceof Error ? e.message : "failed to submit calc"); }
  }, [params]);

  const clear = useCallback(() => { setJob(null); setResult(null); setStats(null); setError(null); resolvedRef.current = null; }, []);

  return { params, setParams, job, result, stats, error, run, clear };
}
```

- [ ] **Step 4: Run → PASS.** Barrel: `export * from "./use-calc";`

- [ ] **Step 5: Commit**

```bash
git add apps/web/src/components/deck/use-calc.ts apps/web/src/components/deck/use-calc.test.ts apps/web/src/components/deck/index.ts
git commit -m "feat(web): headless useCalc hook — per-instance calc job state (no global store)"
```

---

### Task 10: `<CalcControls>` presentational panel

**Files:**
- Create: `apps/web/src/components/deck/calc-controls.tsx`, `.browser.test.tsx`
- Create: `apps/web/src/components/deck/route-mode.ts`, `.test.ts`

**Interfaces:**
- Consumes: `useCalc` return (Task 9); `AREA_MODES`, `ROUTE_INPUT_MODES`, `type CalcMode` from `@/map/lib/calc-request`; `getAlgorithms` from `@/map/data/calc-client`; shadcn `Select`/`Input`/`Button`/`Badge`.
- Produces: `<CalcControls>` props `{ calc: UseCalcReturn; onRun: () => void; disabled?: boolean; disabledReason?: string }` (presentational — the parent owns area/route inputs and calls `calc.run`). And `categoryToRouteMode(category: string): string`.

- [ ] **Step 1: Write failing unit test for `route-mode.ts`**

```ts
// route-mode.test.ts
import { describe, expect, it } from "vitest";
import { categoryToRouteMode } from "./route-mode";

describe("categoryToRouteMode", () => {
  it("maps calc categories to route modes", () => {
    expect(categoryToRouteMode("spawnpoint")).toBe("pokemon");
    expect(categoryToRouteMode("pokestop")).toBe("quest");
    expect(categoryToRouteMode("gym")).toBe("fort");
    expect(categoryToRouteMode("unknown")).toBe("unset");
  });
});
```

> CONFIRMED: `ROUTE_MODES = GEOFENCE_MODES` in `apps/web/src/lib/constants.ts` = the collapsed 4-value set `{unset, pokemon, fort, quest}` (the backend `get_enum` maps legacy `circle_pokemon→Pokemon`, `circle_raid→Fort`, etc.). Map to THESE ids, not the legacy `circle_*` strings.

- [ ] **Step 2: Run → FAIL. Implement `route-mode.ts`:**

```ts
/** Calc golbat category → route mode (v1: category auto-sets the route mode).
 *  Values MUST match ROUTE_MODES ids in lib/constants.ts / the Rust Mode enum. */
const MAP: Record<string, string> = {
  spawnpoint: "pokemon",
  pokestop: "quest",
  gym: "fort",
  fort: "fort",
};
export function categoryToRouteMode(category: string): string {
  return MAP[category] ?? "unset";
}
```

- [ ] **Step 3: Run → PASS.**

- [ ] **Step 4: Write failing browser test for `<CalcControls>`** — renders mode select; clicking "Calculate" calls `onRun`. Port the JSX from `src/map/panels/calc-panel.tsx` but read/write via the `calc` prop instead of the stores.

- [ ] **Step 5: Implement `calc-controls.tsx`** (presentational; mode/category/algorithm/radius/minPoints/sortBy from `calc.params` via `calc.setParams`; progress bar + stats from `calc.job`/`calc.stats`; a "Calculate" button → `onRun`, disabled when `disabled`). Reuse the `Field`/`CalcStats` helpers from the existing panel.

- [ ] **Step 6: Run → PASS.** Barrel: `export * from "./calc-controls"; export * from "./route-mode";`

- [ ] **Step 7: Commit**

```bash
git add apps/web/src/components/deck/calc-controls.tsx apps/web/src/components/deck/calc-controls.browser.test.tsx apps/web/src/components/deck/route-mode.ts apps/web/src/components/deck/route-mode.test.ts apps/web/src/components/deck/index.ts
git commit -m "feat(web): CalcControls panel + category→route-mode map (store-free, driven by useCalc)"
```

---

### Task 11: `<RouteMap>` — reactive fence area + calc → geometry

**Files:**
- Create: `apps/web/src/components/deck/route-map.tsx`, `.browser.test.tsx`

**Interfaces:**
- Consumes: `useWatch`, `useFormContext` from `react-hook-form`; `useGetOne` from `ra-core`; `useCalc`, `CalcControls`, `categoryToRouteMode` (Tasks 9–10); `DeckMap`, `geometryBounds`; `buildBaseLayers` (fence context + result overlay); `featureToAreaFC`, `routeCoordsToClusters`, `AREA_MODES`, `ROUTE_INPUT_MODES` from `@/map/lib/calc-request`; `routeCoords` from `@/map/lib/calc-overlay`.
- Produces: `<RouteMap>` (no props — reads the route form). Behavior per spec §5.

- [ ] **Step 1: Write failing browser test** — set `geofence_id` in the form; assert `useGetOne("geofence", { id })` is invoked and the map renders; simulate a succeeded calc and assert `setValue("geometry", …)` is called with a MultiPoint. Mock `useCalc` + `useGetOne`.

```tsx
// route-map.browser.test.tsx (essential wiring assertions)
import { describe, expect, it, vi } from "vitest";
const setValue = vi.fn();
vi.mock("react-hook-form", async (orig) => {
  const actual = await orig<typeof import("react-hook-form")>();
  return { ...actual, useFormContext: () => ({ setValue, getValues: () => undefined }), useWatch: () => 7 };
});
vi.mock("ra-core", async (orig) => {
  const actual = await orig<typeof import("ra-core")>();
  return { ...actual, useGetOne: vi.fn(() => ({ data: { id: 7, geometry: { type: "Polygon", coordinates: [[[0,0],[1,0],[1,1],[0,1],[0,0]]] } }, isLoading: false })) };
});
// … render <RouteMap/> inside AdminContext; assert deck-map present + useGetOne called with ("geofence", { id: 7 }).
```

- [ ] **Step 2: Run → FAIL.**

- [ ] **Step 3: Implement `route-map.tsx`**

```tsx
import { useEffect, useMemo } from "react";
import { useFormContext, useWatch } from "react-hook-form";
import { useGetOne } from "ra-core";
import type { Layer } from "@deck.gl/core";
import { buildBaseLayers } from "@/map/lib/layers";
import { featureToAreaFC, routeCoordsToClusters, AREA_MODES, ROUTE_INPUT_MODES } from "@/map/lib/calc-request";
import { routeCoords } from "@/map/lib/calc-overlay";
import { DeckMap } from "./deck-map";
import { geometryBounds } from "./bounds";
import { useCalc } from "./use-calc";
import { CalcControls } from "./calc-controls";
import { categoryToRouteMode } from "./route-mode";

export function RouteMap() {
  const form = useFormContext();
  const geofenceId = useWatch({ name: "geofence_id" }) as number | string | undefined;
  const geometry = useWatch({ name: "geometry" }) as GeoJSON.Geometry | null | undefined;

  // Reactive area: the parent fence, re-fetched when geofence_id changes.
  const { data: fence } = useGetOne("geofence", { id: geofenceId! }, { enabled: geofenceId != null });
  const fenceFeature = useMemo<GeoJSON.Feature | null>(
    () => (fence?.geometry ? { type: "Feature", geometry: fence.geometry as GeoJSON.Geometry, properties: {} } : null),
    [fence],
  );

  const calc = useCalc();

  // A succeeded calc result → the route's geometry (MultiPoint of ordered points).
  useEffect(() => {
    if (!calc.result) return;
    const pts = routeCoords(calc.result); // [lon,lat][]
    if (pts.length === 0) return;
    form.setValue("geometry", { type: "MultiPoint", coordinates: pts }, { shouldDirty: true });
    // category auto-sets the route mode for area modes
    if (AREA_MODES.includes(calc.params.mode)) form.setValue("mode", categoryToRouteMode(calc.params.category), { shouldDirty: true });
  }, [calc.result, calc.params.mode, calc.params.category, form]);

  const onRun = () => {
    if (ROUTE_INPUT_MODES.includes(calc.params.mode)) {
      const feat = geometry ? { type: "Feature" as const, geometry, properties: {} } : null;
      void calc.run({ clusters: routeCoordsToClusters(feat) });
    } else {
      if (!fenceFeature) return;
      void calc.run({ area: featureToAreaFC(fenceFeature) });
    }
  };

  const layers = useMemo<Layer[]>(() => {
    const fenceFC: GeoJSON.FeatureCollection = fenceFeature ? { type: "FeatureCollection", features: [fenceFeature] } : { type: "FeatureCollection", features: [] };
    const routeFC: GeoJSON.FeatureCollection = geometry ? { type: "FeatureCollection", features: [{ type: "Feature", geometry, properties: {} }] } : { type: "FeatureCollection", features: [] };
    return buildBaseLayers({
      visibility: { gyms: false, pokestops: false, spawnpoints: false, stations: false, geofences: true, routes: true, s2: false } as Record<string, boolean>,
      markerSets: [],
      geofences: fenceFC, routes: routeFC, s2Cells: [], markerRadius: 70, onClick: () => {}, pickable: false,
      calcResult: calc.result, calcResultIsRoute: true,
    });
  }, [fenceFeature, geometry, calc.result]);

  const fit = geometry ? geometryBounds(geometry) : fenceFeature?.geometry ? geometryBounds(fenceFeature.geometry) : null;
  const areaMissing = AREA_MODES.includes(calc.params.mode) && !fenceFeature;
  const routeMissing = ROUTE_INPUT_MODES.includes(calc.params.mode) && !geometry;

  return (
    <div className="relative">
      <DeckMap layers={layers} fitBounds={fit} height={520} controller={{ doubleClickZoom: true }}>
        <div className="absolute top-2 left-2 z-10">
          <CalcControls calc={calc} onRun={onRun} disabled={areaMissing || routeMissing}
            disabledReason={areaMissing ? "Select a geofence first." : routeMissing ? "Route has no points yet." : undefined} />
        </div>
      </DeckMap>
    </div>
  );
}
```

- [ ] **Step 4: Run → PASS.** Barrel: `export * from "./route-map";`

- [ ] **Step 5: Commit**

```bash
git add apps/web/src/components/deck/route-map.tsx apps/web/src/components/deck/route-map.browser.test.tsx apps/web/src/components/deck/index.ts
git commit -m "feat(web): RouteMap calc workbench — reactive geofence_id area, calc result → route geometry"
```

---

### Task 12: Wire `<RouteMap>` into route-edit; route-show uses the route field

**Files:**
- Modify: `apps/web/src/resources/route/route-edit.tsx`
- Modify test: `apps/web/src/resources/route/route-form.browser.test.tsx`

- [ ] **Step 1: Add a failing assertion** that route-edit renders `getByTestId("deck-map")` and keeps the metadata fields (name/mode/geofence_id).

- [ ] **Step 2: Modify `route-edit.tsx`** — mount the workbench inside the form (the calc workbench replaces the manual `MultiPointInput` as the primary surface; keep the manual input available under a collapsible for the fallback path, or leave it in create-only):

```tsx
import { SimpleForm, TextInput, SelectInput, ReferenceInput } from "@/components/admin";
import { EditLive } from "@/components/realtime";
import { RouteMap } from "@/components/deck";
import { ROUTE_MODES } from "@/lib/constants";
import { required } from "ra-core";
import type { EditProps } from "@/components/admin/views/edit";

export const RouteEdit = (props: Pick<EditProps, "id">) => (
  <EditLive {...props}>
    <SimpleForm>
      <TextInput source="name" validate={required()} />
      <TextInput source="description" />
      <SelectInput source="mode" choices={[...ROUTE_MODES]} />
      <ReferenceInput source="geofence_id" reference="geofence" />
      <RouteMap />
    </SimpleForm>
  </EditLive>
);
```

- [ ] **Step 3: Run route form browser test + typecheck → PASS.**

- [ ] **Step 4: Live-verify** against the real backend (per Global Constraints): open a route edit page, confirm the fence loads from `geofence_id`, run a calc, confirm the result becomes the geometry and Save persists it. Verify by DOM/network, not screenshot.

- [ ] **Step 5: Commit**

```bash
git add apps/web/src/resources/route/route-edit.tsx apps/web/src/resources/route/route-form.browser.test.tsx
git commit -m "feat(web): route-edit is the calc workbench (RouteMap) bound to its geofence"
```

---

## PHASE 5 — Retire the god-map

### Task 13: `<MapIndex>` read-only overworld

**Files:**
- Create: `apps/web/src/components/deck/map-index.tsx`, `.browser.test.tsx`

**Interfaces:**
- Consumes: `useGeoFeatures` from `@/map/data/use-geo-features`; `GeoJsonLayer`; `useNavigate`/`Link` from `react-router`; `DeckMap`.
- Produces: `<MapIndex>` — renders all geofences read-only; click a fence → navigate to `/geofence/{id}`.

- [ ] **Step 1: Write failing browser test** — renders the map; with a stubbed `useGeoFeatures` returning one fence, clicking it navigates. (Wrap in `MemoryRouter`; assert `deck-map` present.)

- [ ] **Step 2: Implement `map-index.tsx`**

```tsx
import { useMemo } from "react";
import { useNavigate } from "react-router";
import { GeoJsonLayer } from "@deck.gl/layers";
import type { Layer, PickingInfo } from "@deck.gl/core";
import { useGeoFeatures } from "@/map/data/use-geo-features";
import { DeckMap } from "./deck-map";
import { geometryBounds } from "./bounds";

export function MapIndex() {
  const navigate = useNavigate();
  const { data } = useGeoFeatures("geofences", true);
  const fc = data ?? { type: "FeatureCollection", features: [] };

  const layers = useMemo<Layer[]>(() => [
    new GeoJsonLayer({
      id: "geofences", data: fc, filled: true,
      getFillColor: [255, 140, 0, 40], getLineColor: [255, 140, 0, 220], lineWidthMinPixels: 1,
      pickable: true,
      onClick: (info: PickingInfo) => {
        const f = info.object as GeoJSON.Feature | undefined;
        const id = f?.id ?? (f?.properties as { id?: string | number } | null)?.id;
        if (id != null) navigate(`/geofence/${id}`);
      },
    }),
  ], [fc, navigate]);

  const fit = fc.features.length ? geometryBounds(fc) : null;
  return <DeckMap layers={layers} fitBounds={fit} height="100%" controller />;
}
```

- [ ] **Step 3: Run → PASS.** Barrel: `export * from "./map-index";`

- [ ] **Step 4: Commit**

```bash
git add apps/web/src/components/deck/map-index.tsx apps/web/src/components/deck/map-index.browser.test.tsx apps/web/src/components/deck/index.ts
git commit -m "feat(web): MapIndex — read-only overworld that navigates into per-fence pages"
```

---

### Task 14: Point `/map` at `<MapIndex>`

**Files:**
- Modify: `apps/web/src/App.tsx`
- Modify test: `apps/web/src/map/map-route.browser.test.tsx` (or add an App-level route test).

- [ ] **Step 1: Add a failing assertion** that `/map` renders the index (a full-bleed `deck-map`, no draw toolbar / calc panel).

- [ ] **Step 2: Modify `App.tsx`** — swap the `/map` element from `<MapRoute />` to a full-bleed `<MapIndex />` wrapper (keep the `<Authenticated>` gate + `noLayout`):

```tsx
import { MapIndex } from "@/components/deck";
// …
<CustomRoutes noLayout>
  <Route
    element={
      <Authenticated>
        <div className="relative h-screen w-screen overflow-hidden">
          <MapIndex />
          <Button asChild size="sm" variant="secondary" className="absolute top-4 left-4 z-10 bg-background/90 shadow-md backdrop-blur">
            <Link to="/"><ArrowLeft className="size-4" /> Admin</Link>
          </Button>
        </div>
      </Authenticated>
    }
    path="/map"
  />
</CustomRoutes>
```

- [ ] **Step 3: Delete the now-orphaned overworld** — remove `src/map/map-route.tsx` and the panels only it used (`draw-toolbar`, `filter-panel`, `calc-panel`, `layer-drawer`, `coordinate-readout`, `selection-popup`, `tile-server-select`), the god-map `deck-canvas.tsx`, and the four global stores IF nothing else imports them. **Before deleting each file, grep for imports** — keep anything still referenced (e.g. `map-settings-store`, `map/lib/*`, `map/data/*` are reused by the deck suite). Deletion is a separate commit so it's easy to revert.

Run: `cd apps/web && rg -l "map/panels/|map/deck-canvas|map/map-route|map/stores/map-ui-store|map/stores/map-view-store|map/stores/map-calc-store" src` — only the files being deleted should match.

- [ ] **Step 4: Run full gate**

Run: `cd apps/web && bun run typecheck && bun run test && bun run test:browser`
Expected: all PASS. Fix any dangling import.

- [ ] **Step 5: Commit**

```bash
git add apps/web/src/App.tsx apps/web/src/map/
git commit -m "feat(web): /map is now a read-only index; retire the overworld god-map + global map stores"
```

---

## Self-Review

**Spec coverage:**
- §4 L0 `<DeckMap>` → Task 1. L1 field → Tasks 2–3. L2 input → Tasks 4–6. L3 GeofenceMap → Task 7; RouteMap → Tasks 9–12. L4 index → Tasks 13–14. ✓
- §5 route calc workbench (reactive `geofence_id`, AREA+ROUTE modes, result→geometry, category→mode) → Tasks 9–12. ✓
- §5 fence→routes linkage → Task 8. ✓
- §6 per-instance state (transient camera, local calc/edit state, keep settings singleton) → Tasks 1/4/9. ✓
- §7 data flow (turf bbox → markers; geofence_id → area; result → geometry) → Tasks 7/11. ✓
- §8 testing (DOM/props not screenshots; RecordContext/RHF harness; stub ctx hooks) → every task's test steps. ✓
- §11 open wire details (`geofence_id` filter param → Task 8 Step 1; category→mode → Task 10; sortBy casing → server default in Task 10). ✓

**Placeholder scan:** Tasks 5, 10 defer some JSX to "port from the existing panel" with explicit source file + prop-mapping instructions — acceptable (the existing file is the concrete reference), but the implementer MUST read `calc-panel.tsx` / `draw-toolbar.tsx` first. Two "confirm the exact enum/union member before finalizing" notes (edit-modes `DrawMode`, `ROUTE_MODES` ids) are real pre-work, not placeholders. No TBD/TODO left.

**Type consistency:** `Bounds` = `[number,number,number,number]` used uniformly (Task 1 re-exports from `map/stores/types`). `useDeckEditRHF` return (`draft/mode/setMode/selectedIndexes/onEdit/onSelect`) matches its consumption in Task 5. `useCalc` return matches `CalcControls` (Task 10) + `RouteMap` (Task 11) usage. `buildBaseLayers` input shape matches Tasks 7/11 (visibility keyed by `LayerId`, cast where the fixed set omits keys). `DrawMode` union flagged for confirmation in Task 5.

## Execution Handoff

**Plan complete and saved to `docs/superpowers/plans/2026-07-09-per-entity-maps.md`.**
