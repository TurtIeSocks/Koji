"use client";

import type * as L from "leaflet";

import {
  ShapeFieldShell,
  type ShapeFieldShellProps,
} from "./shape-field-shell";

type PointFieldProps = Omit<ShapeFieldShellProps, "pathOptions"> & {
  pathOptions?: L.PathOptions;
};

function PointField(props: PointFieldProps) {
  return <ShapeFieldShell {...props} />;
}

export { PointField, type PointFieldProps };
