import { type ReactNode, useMemo } from "react";
import { useRecordContext } from "shadmin-core";
import { GeoJsonLayer, LineLayer } from "@deck.gl/layers";
import type { Layer } from "@deck.gl/core";
import { Button } from "@/components/ui/button";
import { routeCoords, routeSegments, segmentColors, type RouteSegment } from "@/map/lib/calc-overlay";
import { DeckMap } from "./deck-map";
import { useMarkerOverlay } from "./use-marker-overlay";

// Mirrors Leaflet `BaseFieldProps` (components/leaflet/types.ts) for drop-in
// parity, with one deliberate divergence: styling is deck-native RGBA
// `fillColor`/`lineColor` instead of Leaflet's `pathOptions`. A future
// Leaflet→deck backport must translate `pathOptions` → these two props.
export interface DeckGeoJsonFieldProps {
  source: string;
  height?: number | string;
  tileUrl?: string;
  fitBounds?: boolean;
  emptyText?: ReactNode;
  variant?: "geometry" | "route";
  fillColor?: [number, number, number, number];
  lineColor?: [number, number, number, number];
  /** Mode-driven marker overlay (geofence-show passes the fence's mode) —
   *  when set, renders a "Show <label>" toggle that fetches + layers the
   *  mode's golbat categories scoped to `markerArea`. Default OFF. */
  markerMode?: string;
  /** Area to scope the marker fetch to — typically the same geometry as
   *  `source`. Null-tolerant: `useMarkerOverlay` disables fetching until set. */
  markerArea?: GeoJSON.Geometry | null;
  /** Opt in to DeckMap's expand-to-fullscreen button. */
  expandable?: boolean;
}

const DEFAULT_FILL: [number, number, number, number] = [255, 140, 0, 40];
const DEFAULT_LINE: [number, number, number, number] = [255, 140, 0, 220];

export function DeckGeoJsonField({
  source, height = 640, tileUrl, fitBounds = true, emptyText = "No geometry available",
  variant = "geometry", fillColor = DEFAULT_FILL, lineColor = DEFAULT_LINE,
  markerMode, markerArea, expandable,
}: DeckGeoJsonFieldProps) {
  const record = useRecordContext();
  // GeoJSON.GeoJSON (the full union), not GeoJsonObject — matches DeckMap's
  // fitBounds prop + geometryBounds' param so the value flows through untyped-cast.
  const geom = record?.[source] as GeoJSON.GeoJSON | null | undefined;

  // Called unconditionally (rules of hooks) — before the early return below.
  // Null-tolerant: with no markerArea it just stays fetch-disabled.
  const overlay = useMarkerOverlay(markerMode, markerArea ?? null);

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
  const allLayers = markerMode ? [...layers, ...overlay.markerLayers] : layers;
  return (
    <DeckMap
      layers={allLayers}
      fitBounds={fitBounds ? geom : null}
      height={height}
      tileUrl={tileUrl}
      controller={{ doubleClickZoom: true }}
      expandable={expandable}
    >
      {markerMode ? (
        <Button
          type="button"
          size="sm"
          variant="outline"
          className="absolute left-2 top-2 z-10 bg-background/95"
          onClick={() => overlay.setOn(!overlay.on)}
        >
          {overlay.on ? `Hide ${overlay.label}` : `Show ${overlay.label}`}
        </Button>
      ) : null}
    </DeckMap>
  );
}
