"use client";

import {
  ShapeInputShell,
  type ShapeInputShellProps,
} from "./shapes/shape-input-shell";
import type { ShapeKind } from "./types";

interface FeatureInputProps
  extends Omit<
    ShapeInputShellProps,
    "shape" | "multi" | "collection" | "valueTransform" | "valueParse"
  > {
  /** Default geometry type when the user starts drawing. Defaults to `"Polygon"`. */
  shape?: Exclude<ShapeKind, "GeometryCollection">;
}

/**
 * Form input for drawing/editing a `GeoJSON.Feature`. Wraps `ShapeInputShell`
 * but persists `{ type: "Feature", geometry, properties }`, preserving
 * `properties` across edits.
 */
function FeatureInput({ shape = "Polygon", ...rest }: FeatureInputProps) {
  return (
    <ShapeInputShell
      {...rest}
      shape={shape}
      multi={false}
      valueTransform={(geom, prev) => {
        const prevFeat = (prev as GeoJSON.Feature | null | undefined) ?? null;
        const feature: GeoJSON.Feature = {
          type: "Feature",
          geometry: geom,
          properties: prevFeat?.properties ?? {},
        };
        return feature;
      }}
      valueParse={(stored) => {
        const feat = stored as GeoJSON.Feature | null;
        return feat?.geometry ?? null;
      }}
    />
  );
}

export { FeatureInput, type FeatureInputProps };
