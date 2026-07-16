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

/** One lat/lon pair → GeoJSON position, or null. THE FLIP LIVES HERE:
 *  input order is lat,lon (v1 wire habit), GeoJSON is [lon, lat]. */
function parsePair(a: string, b: string): [number, number] | null {
  const lat = Number(a.trim());
  const lon = Number(b.trim());
  if (!Number.isFinite(lat) || !Number.isFinite(lon)) return null;
  if (Math.abs(lat) > 90 || Math.abs(lon) > 180) return null;
  return [lon, lat];
}

/** Port of v1's text sniffing (main:server/model/src/api/text.rs:8-15,55-80):
 *  if the first whitespace token parses as a float, pairs are "lat lon"
 *  separated by commas; otherwise lines are "lat,lon" separated by newlines.
 *  Deviation from v1 (deliberate): v1 silently skipped unparseable pairs; we
 *  error on them — silent point-dropping produces silently-wrong fences.
 *  Output: ONE closed Polygon feature ([lon,lat] order — see parsePair). */
export function parseLatLonText(text: string): ParseOk | ParseErr {
  const trimmed = text.trim();
  if (!trimmed) return { error: "Empty input" };
  const spacePairs = Number.isFinite(Number(trimmed.split(/\s+/)[0]));
  const chunks = trimmed.split(spacePairs ? "," : "\n");
  const ring: [number, number][] = [];
  for (const chunk of chunks) {
    const parts = spacePairs ? chunk.trim().split(/\s+/) : chunk.split(",");
    if (parts.join("").trim() === "") continue; // tolerate blank lines/segments
    if (parts.length < 2) return { error: `Not a lat,lon pair: "${chunk.trim()}"` };
    // Tokens beyond the first two are deliberately dropped: lat,lon,alt is a
    // common scanner-export shape (dragonite/poracle), and v1 read only the
    // first two fields as well.
    const pair = parsePair(parts[0], parts[1]);
    if (!pair) return { error: `Not a valid lat,lon pair: "${chunk.trim()}"` };
    ring.push(pair);
  }
  const closed =
    ring.length > 1 &&
    ring[0][0] === ring[ring.length - 1][0] &&
    ring[0][1] === ring[ring.length - 1][1];
  const distinct = closed ? ring.length - 1 : ring.length;
  if (distinct < 3) return { error: "Need at least 3 lat,lon points for a polygon" };
  const coords = closed ? ring : [...ring, ring[0]];
  return {
    features: [
      { type: "Feature", properties: {}, geometry: { type: "Polygon", coordinates: [coords] } },
    ],
  };
}

/** Single entry point for the Source step: JSON-looking input goes to the
 *  GeoJSON parser (so malformed JSON reports a JSON error), anything else is
 *  tried as a lat,lon list — v1's auto-convert behavior, client-side. */
export function parseSourceText(text: string): ParseOk | ParseErr {
  const t = text.trim();
  if (t.startsWith("{") || t.startsWith("[")) return parseGeoJsonText(text);
  return parseLatLonText(text);
}
