"use client";

import {
  ShapeInputShell,
  type ShapeInputShellProps,
} from "./shape-input-shell";

interface PolygonInputProps
  extends Omit<ShapeInputShellProps, "shape" | "multi" | "geomanShapes"> {
  allowSelfIntersection?: boolean;
  minVertices?: number;
  maxVertices?: number;
  minArea_m2?: number;
}

const POLYGON_TOOLS = ["Polygon", "Rectangle", "Circle"] as const;

function PolygonInput({
  allowSelfIntersection: _allowSelfIntersection,
  minVertices: _minVertices,
  maxVertices: _maxVertices,
  minArea_m2: _minArea_m2,
  ...rest
}: PolygonInputProps) {
  return (
    <ShapeInputShell
      {...rest}
      shape="Polygon"
      multi={false}
      geomanShapes={[...POLYGON_TOOLS]}
    />
  );
}

export { PolygonInput, type PolygonInputProps };
