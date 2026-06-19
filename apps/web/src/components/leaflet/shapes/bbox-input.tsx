"use client";

import {
  ShapeInputShell,
  type ShapeInputShellProps,
} from "./shape-input-shell";
import {
  polygonToBBox,
  bboxToPolygon,
  aspectLockedBBox,
} from "../osm/geometry-ops";

interface BBoxInputProps
  extends Omit<
    ShapeInputShellProps,
    "shape" | "multi" | "valueTransform" | "valueParse"
  > {
  minBBoxArea_m2?: number;
  /**
   * Lock the stored bbox to width/height = ratio. The drawn rectangle is
   * resized to match before persistence. Note: the displayed layer keeps the
   * freely-drawn extent until next hydration.
   */
  aspectRatio?: number;
}

function BBoxInput({
  minBBoxArea_m2: _minBBoxArea_m2,
  aspectRatio,
  ...rest
}: BBoxInputProps) {
  // A drawn rectangle is a `Polygon` in Geoman; we round-trip it through
  // `[w, s, e, n]` (GeoJSON.BBox) for storage in the form value.
  return (
    <ShapeInputShell
      {...rest}
      shape="Polygon"
      multi={false}
      geomanShapes={["Rectangle"]}
      valueTransform={
        aspectRatio ? aspectLockedBBox(aspectRatio) : polygonToBBox
      }
      valueParse={bboxToPolygon}
    />
  );
}

export { BBoxInput, type BBoxInputProps };
