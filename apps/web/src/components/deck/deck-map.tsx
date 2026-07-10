import { type ReactNode, useLayoutEffect, useMemo, useRef, useState } from "react";
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
  const containerRef = useRef<HTMLDivElement>(null);
  // Live container dimensions — fitBounds and onViewStateChange bounds must use
  // the map's OWN rendered size (an embedded 400px field is not window-sized).
  const sizeRef = useRef<{ w: number; h: number }>({ w: 800, h: 600 });

  // Fit once on mount — a stable initial camera. Live camera stays transient.
  // Synchronous path: explicit initialViewState, or no fitBounds → default. A
  // fitBounds needs the container's real size, so it resolves in the layout
  // effect below (`null` until then → DeckGL renders on the next commit, before
  // paint, so no flicker).
  const [initial, setInitial] = useState<ViewState | null>(() => {
    if (initialViewState) return { ...DEFAULT_VS, ...initialViewState };
    const b = fitBounds == null ? null : isBounds(fitBounds) ? fitBounds : geometryBounds(fitBounds);
    return b ? null : DEFAULT_VS;
  });

  useLayoutEffect(() => {
    const el = containerRef.current;
    if (!el) return;
    const measure = () => {
      const r = el.getBoundingClientRect();
      sizeRef.current = { w: r.width || 800, h: r.height || 600 };
      return sizeRef.current;
    };
    const { w, h } = measure();
    // Compute the fit-to-bounds initial view exactly once, now that we know the
    // real dims. Guard on `prev` so a later fitBounds change never fights a user pan.
    setInitial((prev) => {
      if (prev) return prev;
      const b = fitBounds == null ? null : isBounds(fitBounds) ? fitBounds : geometryBounds(fitBounds);
      return b ? { ...DEFAULT_VS, ...boundsToViewState(b, w, h) } : DEFAULT_VS;
    });
    const ro = new ResizeObserver(measure);
    ro.observe(el);
    return () => ro.disconnect();
  }, [fitBounds]);

  const mapStyle = useMemo(() => rasterStyle(tileUrl), [tileUrl]);

  return (
    <div ref={containerRef} data-testid="deck-map" className="relative overflow-hidden rounded-md border" style={{ height, width: "100%" }}>
      {initial ? (
        <DeckGL
          initialViewState={initial}
          controller={controller}
          layers={layers}
          getCursor={getCursor}
          onViewStateChange={(p) => {
            const vs = p.viewState as unknown as ViewState;
            if (!onViewStateChange) return;
            const { w, h } = sizeRef.current;
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
      ) : null}
      {children}
    </div>
  );
}
