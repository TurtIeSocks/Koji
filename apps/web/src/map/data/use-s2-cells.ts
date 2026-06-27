import { useQuery } from "@tanstack/react-query";
import { apiV2Fetch } from "@/lib/http";
import { boundsToBboxArg } from "@/map/lib/coords";
import type { Bounds } from "@/map/stores/types";

export async function fetchS2Cells(level: number, bounds: Bounds): Promise<string[]> {
  const res = await apiV2Fetch(`/s2/${level}`, {
    method: "POST",
    body: JSON.stringify(boundsToBboxArg(bounds)),
  });
  if (res.status < 200 || res.status >= 300) return [];
  const j = res.json as { data?: { id: string }[] } | { id: string }[];
  const cells = Array.isArray(j) ? j : (j.data ?? []);
  return cells.map((c) => c.id);
}

export function useS2Cells(level: number, bounds: Bounds, enabled: boolean) {
  return useQuery({
    queryKey: ["s2", level, bounds],
    queryFn: () => fetchS2Cells(level, bounds),
    enabled,
    staleTime: 60_000,
  });
}
