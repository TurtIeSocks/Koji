import { useQuery } from "@tanstack/react-query";
import { apiV2Fetch } from "@/lib/http";
import type { Bounds } from "@/map/stores/types";

const EMPTY: GeoJSON.FeatureCollection = { type: "FeatureCollection", features: [] };

/** Unwrap an `apiV2Fetch` result into a `FeatureCollection`: accept either the
 *  enveloped `{data}` shape or a raw (non-enveloped) FeatureCollection; a
 *  non-2xx status or anything else unrecognized returns `EMPTY`. */
function unwrapFc(res: { status: number; json: unknown }): GeoJSON.FeatureCollection {
  if (res.status < 200 || res.status >= 300) return EMPTY;
  const j = res.json as { data?: GeoJSON.FeatureCollection; type?: string };
  return j?.data ?? (j?.type === "FeatureCollection" ? (j as GeoJSON.FeatureCollection) : EMPTY);
}

export async function fetchFeatureCollection(
  resource: "geofences" | "routes",
): Promise<GeoJSON.FeatureCollection> {
  const res = await apiV2Fetch(`/${resource}?format=featurecollection`);
  return unwrapFc(res);
}

export function useGeoFeatures(resource: "geofences" | "routes", enabled: boolean) {
  return useQuery({
    queryKey: ["geo", resource],
    queryFn: () => fetchFeatureCollection(resource),
    enabled,
    staleTime: 60_000,
  });
}

/** Scoped geofence-geometry fetch by id list — never fetch-all (the org can
 *  have 600+ fences). Used by the project member-map and the geofence
 *  neighbor overlay. */
export function useGeofencesByIds(ids: (number | string)[], enabled?: boolean) {
  const sortedIds = [...ids].map(String).sort();
  return useQuery({
    queryKey: ["geo", "geofences", "ids", sortedIds],
    queryFn: async () => {
      const res = await apiV2Fetch(`/geofences?format=featurecollection&ids=${ids.join(",")}`);
      return unwrapFc(res);
    },
    enabled: (enabled ?? true) && ids.length > 0,
    staleTime: 60_000,
  });
}

/** Scoped geofence-geometry fetch by viewport bbox — same never-fetch-all
 *  rationale as `useGeofencesByIds`. */
export function useGeofencesByBbox(bbox: Bounds | null, enabled: boolean) {
  return useQuery({
    queryKey: ["geo", "geofences", "bbox", bbox],
    queryFn: async () => {
      const res = await apiV2Fetch(`/geofences?format=featurecollection&bbox=${bbox!.join(",")}`);
      return unwrapFc(res);
    },
    enabled: enabled && !!bbox,
    staleTime: 60_000,
  });
}
