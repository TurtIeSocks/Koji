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
  fitBounds?: GeoJSON.GeoJSON | Bounds | null;
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
  tileUrl = DEFAULT_TILE_URL, onViewStateChange, controller = true,
  getCursor = ({ isDragging }) => (isDragging ? "grabbing" : "grab"), children,
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
