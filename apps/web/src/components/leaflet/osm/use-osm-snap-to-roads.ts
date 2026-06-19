interface SnapToRoadsOptions {
  endpoint?: string;
  profile?: "driving" | "walking" | "cycling";
}

const DEFAULT_ENDPOINT = "https://router.project-osrm.org";

export async function snapToRoadsOnce(
  line: GeoJSON.LineString,
  opts: SnapToRoadsOptions = {},
): Promise<GeoJSON.LineString | null> {
  const endpoint = opts.endpoint ?? DEFAULT_ENDPOINT;
  const profile = opts.profile ?? "driving";
  const coords = line.coordinates.map((c) => `${c[0]},${c[1]}`).join(";");
  const url = `${endpoint}/match/v1/${profile}/${coords}?geometries=geojson&overview=full`;
  const res = await fetch(url);
  if (!res.ok) return null;
  const json = (await res.json()) as {
    code: string;
    matchings: Array<{ geometry: GeoJSON.LineString }>;
  };
  if (json.code !== "Ok" || !json.matchings?.length) return null;
  return json.matchings[0].geometry;
}

export { type SnapToRoadsOptions };
