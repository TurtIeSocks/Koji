import { type ReactNode, useMemo } from "react";
import { useRecordContext } from "shadmin-core";
import { GeoJsonLayer, LineLayer } from "@deck.gl/layers";
import type { Layer } from "@deck.gl/core";
import { routeCoords, routeSegments, segmentColors, type RouteSegment } from "@/map/lib/calc-overlay";
import { DeckMap } from "./deck-map";

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
}

const DEFAULT_FILL: [number, number, number, number] = [255, 140, 0, 40];
const DEFAULT_LINE: [number, number, number, number] = [255, 140, 0, 220];

export function DeckGeoJsonField({
  source, height = 400, tileUrl, fitBounds = true, emptyText = "No geometry available",
  variant = "geometry", fillColor = DEFAULT_FILL, lineColor = DEFAULT_LINE,
}: DeckGeoJsonFieldProps) {
  const record = useRecordContext();
  // GeoJSON.GeoJSON (the full union), not GeoJsonObject — matches DeckMap's
  // fitBounds prop + geometryBounds' param so the value flows through untyped-cast.
  const geom = record?.[source] as GeoJSON.GeoJSON | null | undefined;

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
