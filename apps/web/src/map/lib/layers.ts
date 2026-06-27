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
