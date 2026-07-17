import { apiV2Fetch } from "@/lib/http";
import { boundsToBboxArg } from "@/map/lib/coords";
import type { Bounds, MarkerCategory } from "@/map/stores/types";
import type { TthFilter } from "../types";

interface MarkersBody { points: [number, number][] }

/** Accept either `{status,data:{points}}` (enveloped) or raw `{points}`. */
function readPoints(json: unknown): [number, number][] {
  const j = json as { data?: MarkersBody; points?: [number, number][] };
  return j?.data?.points ?? j?.points ?? [];
}

/** Convert snake_case boundsToBboxArg output to camelCase as required by
 *  the real Rust BboxInput DTO (`serde rename_all = "camelCase"`). */
function toBboxWire(bounds: Bounds): { minLat: number; minLon: number; maxLat: number; maxLon: number } {
  const b = boundsToBboxArg(bounds);
  return { minLat: b.min_lat, minLon: b.min_lon, maxLat: b.max_lat, maxLon: b.max_lon };
}

export async function fetchMarkers(
  category: MarkerCategory,
  area: GeoJSON.Geometry | null,
  bounds: Bounds,
  lastSeen: number,
  tth?: TthFilter,
): Promise<[number, number][]> {
  // Prefer the actual polygon `area` — the `/golbat-data` POST surface runs
  // points_from_area, which filters to the real shape. A MultiPolygon's bbox
  // would wrongly return points in the gaps between its parts. Fall back to the
  // bbox only when there's no geometry.
  const body: Record<string, unknown> = area
    ? { area, lastSeen }
    : { bbox: toBboxWire(bounds), lastSeen };
  if (category === "spawnpoint" && tth && tth !== "All") {
    body.tth = tth;
  }
  const res = await apiV2Fetch(`/golbat-data/${category}`, {
    method: "POST",
    body: JSON.stringify(body),
  });
  if (res.status < 200 || res.status >= 300) return [];
  return readPoints(res.json);
}
