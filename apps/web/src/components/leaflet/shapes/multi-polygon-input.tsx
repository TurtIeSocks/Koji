"use client";

import {
  ShapeInputShell,
  type ShapeInputShellProps,
} from "./shape-input-shell";

interface MultiPolygonInputProps
  extends Omit<ShapeInputShellProps, "shape" | "multi" | "geomanShapes"> {
  allowSelfIntersection?: boolean;
  minVertices?: number;
  maxVertices?: number;
}

function MultiPolygonInput({
  allowSelfIntersection: _allowSelfIntersection,
  minVertices: _minVertices,
  maxVertices: _maxVertices,
  ...rest
}: MultiPolygonInputProps) {
  return (
    <ShapeInputShell
      {...rest}
      shape="MultiPolygon"
      multi={true}
      geomanShapes={["Polygon", "Rectangle", "Circle"]}
    />
  );
}

export { MultiPolygonInput, type MultiPolygonInputProps };
