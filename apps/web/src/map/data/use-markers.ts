import { useQuery } from "@tanstack/react-query";
import { fetchMarkers, type TthFilter } from "@api";
import type { Bounds, MarkerCategory } from "@/map/stores/types";

export function useMarkers(
  category: MarkerCategory,
  area: GeoJSON.Geometry | null,
  bounds: Bounds,
  lastSeen: number,
  enabled: boolean,
  tth?: TthFilter,
) {
  return useQuery({
    queryKey: ["markers", category, area ?? bounds, lastSeen, tth],
    queryFn: () => fetchMarkers(category, area, bounds, lastSeen, tth),
    enabled,
    staleTime: 30_000,
  });
}
