interface GeoJsonFeature {
  type: "Feature";
  geometry: unknown;
  properties: Record<string, unknown> | null;
}
type ParseOk = { features: GeoJsonFeature[] };
type ParseErr = { error: string };

/** Parse raw text into a non-empty list of GeoJSON Features, or a typed error.
 *  Accepts a FeatureCollection or a bare Feature. NEVER silently yields empty —
 *  malformed input and empty collections both return an `error`. */
export function parseGeoJsonText(text: string): ParseOk | ParseErr {
  let parsed: unknown;
  try {
    parsed = JSON.parse(text);
  } catch (e) {
    return { error: `Invalid JSON: ${(e as Error).message}` };
  }
  const obj = parsed as { type?: string; features?: unknown };
  let features: GeoJsonFeature[];
  if (obj?.type === "FeatureCollection" && Array.isArray(obj.features)) {
    features = obj.features as GeoJsonFeature[];
  } else if (obj?.type === "Feature") {
    features = [obj as unknown as GeoJsonFeature];
  } else {
    return { error: "Not a GeoJSON Feature or FeatureCollection" };
  }
  if (features.length === 0) return { error: "No features found in the input" };
  return { features };
}

export type { GeoJsonFeature };
