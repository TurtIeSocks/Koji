"use client";

import {
  ShapeFieldShell,
  type ShapeFieldShellProps,
} from "./shape-field-shell";

type MultiLineStringFieldProps = ShapeFieldShellProps;

function MultiLineStringField(props: MultiLineStringFieldProps) {
  return <ShapeFieldShell {...props} />;
}

export { MultiLineStringField, type MultiLineStringFieldProps };
