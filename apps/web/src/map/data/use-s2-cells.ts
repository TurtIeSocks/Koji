import { useQuery } from "@tanstack/react-query";
import { apiV2Fetch } from "@/lib/http";
import { boundsToBboxArg, fromKojiLatLon } from "@/map/lib/coords";
import type { Bounds, S2Cell } from "@/map/stores/types";

interface RawS2Cell {
  id: string;
  coords: [number, number][]; // Koji [lat, lon] corner ring
}

export async function fetchS2Cells(level: number, bounds: Bounds): Promise<S2Cell[]> {
  const res = await apiV2Fetch(`/s2/${level}`, {
    method: "POST",
    body: JSON.stringify(boundsToBboxArg(bounds)),
  });
  if (res.status < 200 || res.status >= 300) return [];
  const j = res.json as { data?: RawS2Cell[] } | RawS2Cell[];
  const cells = Array.isArray(j) ? j : (j.data ?? []);
  // Render the cell's corner polygon (server sends [lat,lon] → transpose).
  // The numeric `id` is a cell id, NOT an S2 token, so it can't drive S2Layer.
  return cells.map((c) => ({ id: c.id, ring: (c.coords ?? []).map(fromKojiLatLon) }));
}

export function useS2Cells(level: number, bounds: Bounds, enabled: boolean) {
  return useQuery({
    queryKey: ["s2", level, bounds],
    queryFn: () => fetchS2Cells(level, bounds),
    enabled,
    staleTime: 60_000,
  });
}
