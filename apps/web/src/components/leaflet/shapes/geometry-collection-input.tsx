"use client";

import { GeoJsonInput, type GeoJsonInputProps } from "../geojson-input";
import type { ShapeKind } from "../types";

interface GeometryCollectionInputProps
  extends Omit<GeoJsonInputProps, "shapes" | "collection"> {
  allowedShapes?: ShapeKind[];
}

const DEFAULT_ALLOWED: ShapeKind[] = ["Point", "LineString", "Polygon"];

function GeometryCollectionInput({
  allowedShapes = DEFAULT_ALLOWED,
  ...rest
}: GeometryCollectionInputProps) {
  return <GeoJsonInput {...rest} shapes={allowedShapes} collection />;
}

export { GeometryCollectionInput, type GeometryCollectionInputProps };
