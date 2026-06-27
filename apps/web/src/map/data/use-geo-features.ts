import { useQuery } from "@tanstack/react-query";
import { apiV2Fetch } from "@/lib/http";

const EMPTY: GeoJSON.FeatureCollection = { type: "FeatureCollection", features: [] };

export async function fetchFeatureCollection(
  resource: "geofences" | "routes",
): Promise<GeoJSON.FeatureCollection> {
  const res = await apiV2Fetch(`/${resource}?format=featurecollection`);
  if (res.status < 200 || res.status >= 300) return EMPTY;
  const j = res.json as { data?: GeoJSON.FeatureCollection; type?: string };
  return j?.data ?? (j?.type === "FeatureCollection" ? (j as GeoJSON.FeatureCollection) : EMPTY);
}

export function useGeoFeatures(resource: "geofences" | "routes", enabled: boolean) {
  return useQuery({
    queryKey: ["geo", resource],
    queryFn: () => fetchFeatureCollection(resource),
    enabled,
    staleTime: 60_000,
  });
}
