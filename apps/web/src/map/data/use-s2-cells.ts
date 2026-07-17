import { useQuery } from "@tanstack/react-query";
import { fetchS2Cells } from "@api";
import type { Bounds } from "@/map/stores/types";

export function useS2Cells(level: number, bounds: Bounds, enabled: boolean) {
  return useQuery({
    queryKey: ["s2", level, bounds],
    queryFn: () => fetchS2Cells(level, bounds),
    enabled,
    staleTime: 60_000,
  });
}
