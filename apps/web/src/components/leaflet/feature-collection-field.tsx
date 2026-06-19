"use client";

import {
  ShapeFieldShell,
  type ShapeFieldShellProps,
} from "./shapes/shape-field-shell";

/**
 * Read-only Leaflet map that renders a `GeoJSON.FeatureCollection` stored at
 * a record field. `L.geoJSON` renders the collection as a single multi-layer
 * group, so this delegates to `ShapeFieldShell`.
 */
type FeatureCollectionFieldProps = ShapeFieldShellProps;

function FeatureCollectionField(props: FeatureCollectionFieldProps) {
  return <ShapeFieldShell {...props} />;
}

export { FeatureCollectionField, type FeatureCollectionFieldProps };
