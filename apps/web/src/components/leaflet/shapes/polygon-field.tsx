"use client";

import {
  ShapeFieldShell,
  type ShapeFieldShellProps,
} from "./shape-field-shell";

type PolygonFieldProps = ShapeFieldShellProps;

function PolygonField(props: PolygonFieldProps) {
  return <ShapeFieldShell {...props} />;
}

export { PolygonField, type PolygonFieldProps };
