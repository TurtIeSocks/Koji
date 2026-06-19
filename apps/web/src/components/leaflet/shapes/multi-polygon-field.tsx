"use client";

import {
  ShapeFieldShell,
  type ShapeFieldShellProps,
} from "./shape-field-shell";

type MultiPolygonFieldProps = ShapeFieldShellProps;

function MultiPolygonField(props: MultiPolygonFieldProps) {
  return <ShapeFieldShell {...props} />;
}

export { MultiPolygonField, type MultiPolygonFieldProps };
