import { ScatterplotLayer, GeoJsonLayer, PolygonLayer, LineLayer } from "@deck.gl/layers";
import type { Layer, PickingInfo } from "@deck.gl/core";
import type { LayerId, S2Cell } from "@/map/stores/types";
import { packMarkers } from "@/map/lib/coords";
import { EditableGeoJsonLayer } from "@deck.gl-community/editable-layers";
import { modeSpecFor, type DrawMode } from "@/map/lib/edit-modes";
import { routeCoords, routeSegments, segmentColors, type RouteSegment } from "@/map/lib/calc-overlay";

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
  /** The geofence currently open in the editor — its ORIGINAL is drawn dimmed so
   *  it's distinguishable from the other (orange) geofences and the blue draft. */
  editingGeofenceId?: string | null;
}

const GEOFENCE_FILL: [number, number, number, number] = [255, 140, 0, 40];
const GEOFENCE_LINE: [number, number, number, number] = [255, 140, 0, 220];
const EDITING_FILL: [number, number, number, number] = [130, 130, 130, 25];
const EDITING_LINE: [number, number, number, number] = [130, 130, 130, 140];

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
      stroked: true, getLineColor: [0, 200, 120, 220], lineWidthMinPixels: 2,
      pointType: "circle", getPointRadius: 8, pointRadiusUnits: "pixels",
      pickable, onClick,
    }),
    new PolygonLayer<S2Cell>({
      id: "s2", visible: visibility.s2, data: s2Cells,
      getPolygon: (d) => d.ring, filled: false, stroked: true,
      getLineColor: [255, 0, 0, 160], lineWidthMinPixels: 1,
    }),
    ...markerLayers,
    ...(input.calcResult ? calcResultLayers(input.calcResult, input.calcResultIsRoute ?? false) : []),
  ];
}

/** Calc result overlay: magenta centers as dots, plus (for route-family results)
 *  the ordered path connecting them, each leg colored green→red by its length. */
function calcResultLayers(fc: GeoJSON.FeatureCollection, isRoute: boolean): Layer[] {
  const dots = new GeoJsonLayer({
    id: "calc-result", data: fc,
    stroked: true, filled: true,
    getFillColor: [255, 0, 200, 200], getLineColor: [255, 0, 200, 230],
    lineWidthMinPixels: 2,
    pointType: "circle", getPointRadius: 5, pointRadiusUnits: "pixels",
    pickable: false,
  });
  if (!isRoute) return [dots];
  const segs = routeSegments(routeCoords(fc));
  if (segs.length === 0) return [dots];
  const colors = segmentColors(segs);
  const path = new LineLayer<RouteSegment>({
    id: "calc-route",
    data: segs,
    getSourcePosition: (s) => s.source,
    getTargetPosition: (s) => s.target,
    getColor: (_s, info) => colors[info.index],
    getWidth: 3, widthUnits: "pixels", widthMinPixels: 2,
    pickable: false,
  });
  return [path, dots]; // path drawn under the dots
}

export function buildEditLayer(draft: DraftInput | undefined): Layer[] {
  if (!draft || draft.mode === "none") return [];
  const spec = modeSpecFor(draft.mode);
  return [
    new EditableGeoJsonLayer({
      id: "edit",
      data: draft.features,
      mode: spec.ModeClass,
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
