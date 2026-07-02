import { ScatterplotLayer, GeoJsonLayer, PolygonLayer } from "@deck.gl/layers";
import type { Layer, PickingInfo } from "@deck.gl/core";
import type { LayerId, S2Cell } from "@/map/stores/types";
import { packMarkers } from "@/map/lib/coords";
import { EditableGeoJsonLayer } from "@deck.gl-community/editable-layers";
import { modeSpecFor, type DrawMode } from "@/map/lib/edit-modes";

interface MarkerSet { id: LayerId; points: [number, number][]; color: [number, number, number]; }

export interface DraftInput {
  mode: DrawMode;
  features: GeoJSON.FeatureCollection;
  selectedIndexes: number[];
  onEdit: (e: { updatedData: GeoJSON.FeatureCollection; editType?: string }) => void;
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
      getRadius: markerRadius,
      radiusUnits: "meters",
      radiusMinPixels: 2,
      getFillColor: [...set.color, 200],
      pickable,
      onClick,
    });
  });

  return [
    new GeoJsonLayer({
      id: "geofences", visible: visibility.geofences, data: geofences,
      filled: true, getFillColor: [255, 140, 0, 40], getLineColor: [255, 140, 0, 220],
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
    // Calc result overlay (magenta): MultiPoint centers as dots + any route line.
    ...(input.calcResult
      ? [
          new GeoJsonLayer({
            id: "calc-result",
            data: input.calcResult,
            stroked: true, filled: true,
            getFillColor: [255, 0, 200, 180], getLineColor: [255, 0, 200, 220],
            lineWidthMinPixels: 2,
            pointType: "circle", getPointRadius: 5, pointRadiusUnits: "pixels",
            pickable: false,
          }),
        ]
      : []),
  ];
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
