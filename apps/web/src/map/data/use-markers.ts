import { useQuery } from "@tanstack/react-query";
import { apiV2Fetch } from "@/lib/http";
import { boundsToBboxArg } from "@/map/lib/coords";
import type { Bounds, MarkerCategory } from "@/map/stores/types";

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
  bounds: Bounds,
  lastSeen: number,
): Promise<[number, number][]> {
  const res = await apiV2Fetch(`/golbat-data/${category}`, {
    method: "POST",
    body: JSON.stringify({ bbox: toBboxWire(bounds), lastSeen }),
  });
  if (res.status < 200 || res.status >= 300) return [];
  return readPoints(res.json);
}

export function useMarkers(
  category: MarkerCategory,
  bounds: Bounds,
  lastSeen: number,
  enabled: boolean,
) {
  return useQuery({
    queryKey: ["markers", category, bounds, lastSeen],
    queryFn: () => fetchMarkers(category, bounds, lastSeen),
    enabled,
    staleTime: 30_000,
  });
}
