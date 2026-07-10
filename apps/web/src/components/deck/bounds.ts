import bbox from "@turf/bbox";
import type { AllGeoJSON } from "@turf/helpers";
import { WebMercatorViewport } from "@deck.gl/core";
import type { Bounds } from "@/map/stores/types";

export type { Bounds };

/** Turf bbox → deck Bounds order [minLng,minLat,maxLng,maxLat]. Null when the
 *  geometry has no finite extent (empty collection, no coords).
 *
 *  Deviation from plan: the plan's signature took `GeoJSON.GeoJsonObject` and
 *  cast to a nonexistent `GeoJSON.AllGeoJSON`. `@types/geojson`'s
 *  `GeoJsonObject` is the base interface (just `type`/`bbox`) and rejects
 *  concrete literals like `GeometryCollection` via excess-property checks;
 *  `AllGeoJSON` only exists on `@turf/helpers`, not the `geojson` package. Use
 *  `GeoJSON.GeoJSON` (the real union of Feature/FeatureCollection/Geometry/
 *  GeometryCollection) for the param, matching what turf's `bbox()` accepts. */
export function geometryBounds(geom: GeoJSON.GeoJSON): Bounds | null {
  try {
    const b = bbox(geom as AllGeoJSON);
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
