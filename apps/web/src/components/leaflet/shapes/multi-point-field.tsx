"use client";

import {
  ShapeFieldShell,
  type ShapeFieldShellProps,
} from "./shape-field-shell";

type MultiPointFieldProps = ShapeFieldShellProps;

function MultiPointField(props: MultiPointFieldProps) {
  return <ShapeFieldShell {...props} />;
}

export { MultiPointField, type MultiPointFieldProps };
