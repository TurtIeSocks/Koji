import { useQuery } from "@tanstack/react-query";
import {
  fetchFeatureCollection,
  fetchGeofencesByBbox,
  fetchGeofencesByIds,
} from "@api";
import type { Bounds } from "@/map/stores/types";

// Historical home of this type — consumers still import it from here.
export type { NeighborFilters } from "@/api/types";
import type { NeighborFilters } from "@/api/types";

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
    queryFn: () => fetchGeofencesByIds(ids),
    enabled: (enabled ?? true) && ids.length > 0,
    staleTime: 60_000,
  });
}

/** Scoped geofence-geometry fetch by viewport bbox — same never-fetch-all
 *  rationale as `useGeofencesByIds`. Optional mode/projects filters are
 *  server-side bbox refinements (see /v2/geofences ReadQuery). */
export function useGeofencesByBbox(
  bbox: Bounds | null,
  enabled: boolean,
  filters?: NeighborFilters,
) {
  const mode = filters?.mode;
  const projects = filters?.projects?.length ? [...filters.projects].sort() : undefined;
  return useQuery({
    queryKey: ["geo", "geofences", "bbox", bbox, mode ?? null, projects ?? null],
    queryFn: () => fetchGeofencesByBbox(bbox!, { mode, projects }),
    enabled: enabled && !!bbox,
    staleTime: 60_000,
  });
}
