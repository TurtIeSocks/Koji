/** Geofence geometry <-> editable-features conversion for the deck editor.
 *
 *  A geofence is a Polygon OR MultiPolygon. To edit each part independently on
 *  the map we expand it into one Polygon feature per part; to persist we combine
 *  those parts back into a single MultiPolygon (matching the Leaflet
 *  MultiPolygonInput: a lone polygon still saves as a 1-part MultiPolygon, and
 *  the backend accepts either type). */

/** MultiPolygon/Polygon geometry → one editable Polygon feature per part. */
export function geofenceToFeatures(value: unknown): GeoJSON.Feature[] {
  const g = value as GeoJSON.Geometry | null | undefined;
  if (!g) return [];
  if (g.type === "MultiPolygon") {
    return g.coordinates.map((coordinates) => ({
      type: "Feature",
      geometry: { type: "Polygon", coordinates },
      properties: {},
    }));
  }
  // Polygon (or any other single geometry) → one feature as-is.
  return [{ type: "Feature", geometry: g, properties: {} }];
}

/** Edited features → a single MultiPolygon (one part per drawn Polygon). Falls
 *  back to the first feature's geometry when nothing is a Polygon. */
export function featuresToGeofence(features: GeoJSON.Feature[]): unknown {
  const polygons = features
    .map((f) => f.geometry)
    .filter((geom): geom is GeoJSON.Polygon => geom?.type === "Polygon");
  if (polygons.length === 0) return features[0]?.geometry ?? null;
  return {
    type: "MultiPolygon",
    coordinates: polygons.map((p) => p.coordinates),
  } satisfies GeoJSON.MultiPolygon;
}
