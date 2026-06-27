export function firstGeometry(fc: GeoJSON.FeatureCollection): GeoJSON.Geometry | null {
  return fc.features[0]?.geometry ?? null;
}

/** Every drawn feature's geometry — so Save persists ALL shapes, not just the first. */
export function allGeometries(fc: GeoJSON.FeatureCollection): GeoJSON.Geometry[] {
  return fc.features.map((f) => f.geometry).filter((g): g is GeoJSON.Geometry => g != null);
}
