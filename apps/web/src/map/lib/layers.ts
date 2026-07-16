import { ScatterplotLayer, GeoJsonLayer, PolygonLayer, LineLayer } from "@deck.gl/layers";
import type { Layer, PickingInfo } from "@deck.gl/core";
import type { LayerId, S2Cell } from "@/map/stores/types";
import { packMarkers } from "@/map/lib/coords";
import { EditableGeoJsonLayer } from "@deck.gl-community/editable-layers";
import { modeSpecFor, type DrawMode } from "@/map/lib/edit-modes";
import { routeCoords, routeSegments, segmentColors, type RouteSegment } from "@/map/lib/calc-overlay";
import { COLOR } from "@/map/lib/map-colors";

interface MarkerSet { id: LayerId; points: [number, number][]; color: [number, number, number]; radius?: number; maxPixels?: number; }

export interface DraftInput {
  mode: DrawMode;
  features: GeoJSON.FeatureCollection;
  selectedIndexes: number[];
  onEdit: (e: {
    updatedData: GeoJSON.FeatureCollection;
    editType?: string;
    editContext?: { featureIndexes?: number[] };
  }) => void;
  /** Click-to-select while in modify/transform so those modes have a target. */
  onSelect?: (indexes: number[]) => void;
  /** Bumped on a programmatic feature delete so the edit layer's `id` changes and
   *  deck mounts a fresh EditableGeoJsonLayer — it otherwise caches its internal
   *  FeatureCollection and won't reflect an external delete until a pointer event. */
  version?: number;
  /** Pre-built mode instance (createModeInstance). Falls back to the class. */
  modeInstance?: unknown;
}

/** The data layers (markers/geofences/routes/s2). Kept separate from the edit
 *  layer so a per-click draft change only rebuilds the cheap edit layer, not the
 *  marker Float32Array repack. */
export interface BaseLayersInput {
  visibility: Record<LayerId, boolean>;
  markerSets: MarkerSet[];
  geofences: GeoJSON.FeatureCollection;
  routes: GeoJSON.FeatureCollection;
  s2Cells: S2Cell[];
  markerRadius: number;
  onClick: (info: PickingInfo) => void;
  /** When false, base layers skip the picking pass. Turn OFF while drawing so
   *  every pointer-move doesn't re-render + readPixels all the heavy layers. */
  pickable?: boolean;
  /** Calc-job result (cluster centers + route order) to overlay, if any. */
  calcResult?: GeoJSON.FeatureCollection | null;
  /** True for route-family results (route/reroute/route-stats) → draw the ordered
   *  path connecting the centers, colored by leg length. */
  calcResultIsRoute?: boolean;
  /** When set, draw each calc cluster center as a circle of THIS radius in METERS
   *  (the calc radius) — so you can verify which points fall inside each cluster.
   *  Omit for non-radius strategies (e.g. S2). */
  calcResultRadius?: number;
  /** The geofence currently open in the editor — its ORIGINAL is drawn dimmed so
   *  it's distinguishable from the other (orange) geofences and the blue draft. */
  editingGeofenceId?: string | null;
}

export const GEOFENCE_FILL: [number, number, number, number] = [...COLOR.geofence, 40];
export const GEOFENCE_LINE: [number, number, number, number] = [...COLOR.geofence, 220];
export const EDITING_FILL: [number, number, number, number] = [130, 130, 130, 25];
export const EDITING_LINE: [number, number, number, number] = [130, 130, 130, 140];

/** Is this geofence feature the one currently open in the editor? (id lives at the
 *  geojson top-level `feature.id`; fall back to properties.id.) */
function isEditingOriginal(f: GeoJSON.Feature, editingId: string | null | undefined): boolean {
  if (editingId == null) return false;
  const id = f.id ?? (f.properties as { id?: string | number } | null)?.id;
  return id != null && String(id) === editingId;
}

export interface BuildLayersInput extends BaseLayersInput {
  draft?: DraftInput;
}

export function buildBaseLayers(input: BaseLayersInput): Layer[] {
  const { visibility, markerSets, geofences, routes, s2Cells, markerRadius, onClick } = input;
  const pickable = input.pickable ?? true;

  const markerLayers = markerSets.map((set) => {
    const positions = packMarkers(set.points);
    return new ScatterplotLayer({
      id: `markers-${set.id}`,
      visible: visibility[set.id],
      data: { length: set.points.length, attributes: { getPosition: { value: positions, size: 2 } } },
      // Radius is in METERS → the GPU scales it with zoom for free (no re-render).
      // radiusMinPixels/radiusMaxPixels bound it: stays visible when zoomed out,
      // and caps at maxPixels when zoomed in so dense markers stop overlapping
      // (past the cap, zooming in spreads the points but not the dots → de-crowds).
      getRadius: set.radius ?? markerRadius,
      radiusUnits: "meters",
      radiusMinPixels: 1,
      radiusMaxPixels: set.maxPixels,
      getFillColor: [...set.color, 200],
      pickable,
      onClick,
    });
  });

  return [
    new GeoJsonLayer({
      id: "geofences", visible: visibility.geofences, data: geofences,
      filled: true,
      // The one being edited → dimmed; every other geofence → orange.
      getFillColor: (f: GeoJSON.Feature) => (isEditingOriginal(f, input.editingGeofenceId) ? EDITING_FILL : GEOFENCE_FILL),
      getLineColor: (f: GeoJSON.Feature) => (isEditingOriginal(f, input.editingGeofenceId) ? EDITING_LINE : GEOFENCE_LINE),
      // Accessors are memoized by deck — re-evaluate them when the edit target changes.
      updateTriggers: { getFillColor: input.editingGeofenceId, getLineColor: input.editingGeofenceId },
      lineWidthMinPixels: 1, pickable, onClick,
    }),
    new GeoJsonLayer({
      id: "routes", visible: visibility.routes, data: routes,
      stroked: true, getLineColor: [...COLOR.route, 220], lineWidthMinPixels: 2,
      pointType: "circle", getPointRadius: 3, pointRadiusUnits: "pixels",
      getPointFillColor: [...COLOR.route, 230],
      pickable, onClick,
    }),
    new PolygonLayer<S2Cell>({
      id: "s2", visible: visibility.s2, data: s2Cells,
      getPolygon: (d) => d.ring, filled: false, stroked: true,
      getLineColor: [...COLOR.s2, 160], lineWidthMinPixels: 1,
    }),
    ...markerLayers,
    ...(input.calcResult ? calcResultLayers(input.calcResult, input.calcResultIsRoute ?? false, input.calcResultRadius) : []),
  ];
}

/** Calc result overlay: the ordered route path (route-family results), the cluster
 *  coverage circles at the exact calc radius (when `radius` is given), and the
 *  cluster centers as small dots. Bottom → top: path, circles, dots. */
/** The ordered route as distance-colored segments (green→red by leg length) —
 *  the "distance lines" connecting each point. Null for <2 points. Reused by both
 *  the calc-result overlay and the loaded-route path. */
export function routePathLayer(id: string, coords: [number, number][]): Layer | null {
  const segs = routeSegments(coords);
  if (segs.length === 0) return null;
  const colors = segmentColors(segs);
  return new LineLayer<RouteSegment>({
    id,
    data: segs,
    getSourcePosition: (s) => s.source,
    getTargetPosition: (s) => s.target,
    getColor: (_s, info) => colors[info.index],
    getWidth: 3, widthUnits: "pixels", widthMinPixels: 2,
    pickable: false,
  });
}

function calcResultLayers(fc: GeoJSON.FeatureCollection, isRoute: boolean, radius?: number): Layer[] {
  const centers = routeCoords(fc);
  const out: Layer[] = [];

  if (isRoute) {
    const path = routePathLayer("calc-route", centers);
    if (path) out.push(path);
  }

  // Coverage circles at the EXACT calc radius in METERS — stroked ring + faint
  // fill so points inside show through. radiusMinPixels 0 (no floor) keeps the
  // meters exact at every zoom; no max cap for the same reason.
  if (radius && radius > 0) {
    out.push(new ScatterplotLayer<[number, number]>({
      id: "calc-circles",
      data: centers,
      getPosition: (d) => d,
      getRadius: radius,
      radiusUnits: "meters",
      radiusMinPixels: 0,
      stroked: true, filled: true,
      getFillColor: [...COLOR.calcCenter, 25],
      getLineColor: [...COLOR.calcCenter, 220],
      lineWidthMinPixels: 1,
      pickable: false,
      updateTriggers: { getRadius: radius },
    }));
  }

  out.push(new GeoJsonLayer({
    id: "calc-result", data: fc,
    stroked: true, filled: true,
    getFillColor: [...COLOR.calcCenter, 200], getLineColor: [...COLOR.calcCenter, 230],
    lineWidthMinPixels: 2,
    // Cluster-center dots kept small so the coverage circles + underlying markers
    // stay readable (they were 3–5 px "blobs").
    pointType: "circle", getPointRadius: radius ? 1.5 : 2.5, pointRadiusUnits: "pixels",
    pickable: false,
  }));

  return out;
}

export function buildEditLayer(draft: DraftInput | undefined): Layer[] {
  if (!draft || draft.mode === "none") return [];
  const spec = modeSpecFor(draft.mode);
  return [
    new EditableGeoJsonLayer({
      id: `edit-${draft.version ?? 0}`,
      data: draft.features,
      mode: (draft.modeInstance ?? spec.ModeClass) as never,
      // Boolean op (e.g. cut-hole = 'difference') applied against the selection.
      modeConfig: spec.modeConfig,
      selectedFeatureIndexes: draft.selectedIndexes,
      onEdit: draft.onEdit,
      // Click a feature to select it so modify/transform have a target. Draw
      // modes (incl. cut-hole/split) leave this off — their clicks draw.
      onClick: (info: PickingInfo) => {
        if (!spec.clickToSelect || !draft.onSelect) return;
        const i = info.index;
        draft.onSelect(typeof i === "number" && i >= 0 ? [i] : []);
      },
      getFillColor: [0, 150, 255, 60],
      getLineColor: [0, 150, 255, 220],
    }) as unknown as Layer,
  ];
}

export function buildLayers(input: BuildLayersInput): Layer[] {
  return [...buildBaseLayers(input), ...buildEditLayer(input.draft)];
}
