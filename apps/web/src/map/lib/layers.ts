import { ScatterplotLayer, GeoJsonLayer, PolygonLayer } from "@deck.gl/layers";
import type { Layer, PickingInfo } from "@deck.gl/core";
import type { LayerId, S2Cell } from "@/map/stores/types";
import { packMarkers } from "@/map/lib/coords";
import { EditableGeoJsonLayer } from "@deck.gl-community/editable-layers";
import { editModeFor, type DrawMode } from "@/map/lib/edit-modes";

interface MarkerSet { id: LayerId; points: [number, number][]; color: [number, number, number]; }

export interface DraftInput {
  mode: DrawMode;
  features: GeoJSON.FeatureCollection;
  selectedIndexes: number[];
  onEdit: (e: { updatedData: GeoJSON.FeatureCollection }) => void;
  /** Click-to-select while in modify/translate so those modes have a target. */
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
}

export interface BuildLayersInput extends BaseLayersInput {
  draft?: DraftInput;
}

export function buildBaseLayers(input: BaseLayersInput): Layer[] {
  const { visibility, markerSets, geofences, routes, s2Cells, markerRadius, onClick } = input;

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
    new PolygonLayer<S2Cell>({
      id: "s2", visible: visibility.s2, data: s2Cells,
      getPolygon: (d) => d.ring, filled: false, stroked: true,
      getLineColor: [255, 0, 0, 160], lineWidthMinPixels: 1,
    }),
    ...markerLayers,
  ];
}

export function buildEditLayer(draft: DraftInput | undefined): Layer[] {
  if (!draft || draft.mode === "none") return [];
  const canSelect = draft.mode === "modify" || draft.mode === "translate";
  return [
    new EditableGeoJsonLayer({
      id: "edit",
      data: draft.features,
      mode: editModeFor(draft.mode),
      selectedFeatureIndexes: draft.selectedIndexes,
      onEdit: draft.onEdit,
      // Click a feature to select it so modify/translate have a target.
      onClick: (info: PickingInfo) => {
        if (!canSelect || !draft.onSelect) return;
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
