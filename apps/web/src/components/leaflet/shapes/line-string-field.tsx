"use client";

import {
  ShapeFieldShell,
  type ShapeFieldShellProps,
} from "./shape-field-shell";

type LineStringFieldProps = ShapeFieldShellProps;

function LineStringField(props: LineStringFieldProps) {
  return <ShapeFieldShell {...props} />;
}

export { LineStringField, type LineStringFieldProps };
