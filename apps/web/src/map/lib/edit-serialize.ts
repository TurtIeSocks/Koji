export function firstGeometry(fc: GeoJSON.FeatureCollection): GeoJSON.Geometry | null {
  return fc.features[0]?.geometry ?? null;
}
