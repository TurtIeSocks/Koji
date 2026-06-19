"use client";

import {
  ShapeFieldShell,
  type ShapeFieldShellProps,
} from "./shape-field-shell";

type GeometryCollectionFieldProps = ShapeFieldShellProps;

function GeometryCollectionField(props: GeometryCollectionFieldProps) {
  return <ShapeFieldShell {...props} />;
}

export { GeometryCollectionField, type GeometryCollectionFieldProps };
