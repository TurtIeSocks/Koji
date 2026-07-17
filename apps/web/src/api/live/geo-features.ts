import { apiV2Fetch } from "@/lib/http";
import type { Bounds } from "@/map/stores/types";
import type { NeighborFilters } from "../types";

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

/** Scoped geofence-geometry fetch by id list — never fetch-all (the org can
 *  have 600+ fences). */
export async function fetchGeofencesByIds(
  ids: (number | string)[],
): Promise<GeoJSON.FeatureCollection> {
  const res = await apiV2Fetch(`/geofences?format=featurecollection&ids=${ids.join(",")}`);
  return unwrapFc(res);
}

/** Scoped geofence-geometry fetch by viewport bbox — same never-fetch-all
 *  rationale as `fetchGeofencesByIds`. Optional mode/projects filters are
 *  server-side bbox refinements (see /v2/geofences ReadQuery). */
export async function fetchGeofencesByBbox(
  bbox: Bounds,
  filters?: NeighborFilters,
): Promise<GeoJSON.FeatureCollection> {
  let url = `/geofences?format=featurecollection&bbox=${bbox.join(",")}`;
  if (filters?.mode) url += `&mode=${encodeURIComponent(filters.mode)}`;
  if (filters?.projects) url += `&projects=${filters.projects.join(",")}`;
  const res = await apiV2Fetch(url);
  return unwrapFc(res);
}
