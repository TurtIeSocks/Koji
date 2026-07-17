import type { Bounds } from "@/map/stores/types";
import type { NeighborFilters } from "../types";
import type { GeofenceRow } from "./db";
import { allRows } from "./db";
import { ensureSeeded } from "./seeds/seed";

/** One GeoJSON Feature per stored geo row, carrying the minimal overlay
 *  properties (`id`/`name`/`mode`) the map layers read. */
function rowToFeature(row: {
  id: number;
  name: string;
  mode: string;
  geometry: GeoJSON.Geometry;
}): GeoJSON.Feature {
  return {
    type: "Feature",
    id: row.id,
    geometry: row.geometry,
    properties: { id: row.id, name: row.name, mode: row.mode },
  };
}

function collection(features: GeoJSON.Feature[]): GeoJSON.FeatureCollection {
  return { type: "FeatureCollection", features };
}

/** All rows of a geo resource as a FeatureCollection — the demo mirror of
 *  `GET /api/v2/{resource}?format=featurecollection`. */
export async function fetchFeatureCollection(
  resource: "geofences" | "routes",
): Promise<GeoJSON.FeatureCollection> {
  await ensureSeeded();
  const rows = await allRows(resource);
  return collection(rows.map(rowToFeature));
}

/** Scoped geofence-geometry fetch by id list — the demo mirror of
 *  `?ids=`. Ids may arrive as numbers or strings; compared numerically. */
export async function fetchGeofencesByIds(
  ids: (number | string)[],
): Promise<GeoJSON.FeatureCollection> {
  await ensureSeeded();
  const want = new Set(ids.map((id) => Number(id)));
  const rows = await allRows("geofences");
  return collection(rows.filter((r) => want.has(r.id)).map(rowToFeature));
}

function passesFilters(row: GeofenceRow, filters?: NeighborFilters): boolean {
  if (filters?.mode && row.mode !== filters.mode) return false;
  if (filters?.projects && filters.projects.length > 0) {
    if (!row.projects.some((p) => filters.projects!.includes(p))) return false;
  }
  return true;
}

/** Scoped geofence-geometry fetch by viewport bbox — in-memory AABB intersect
 *  on each row's stored bbox fields, plus the optional mode/projects
 *  refinements. `bbox` is `[minLon, minLat, maxLon, maxLat]` (Bounds). */
export async function fetchGeofencesByBbox(
  bbox: Bounds,
  filters?: NeighborFilters,
): Promise<GeoJSON.FeatureCollection> {
  await ensureSeeded();
  const [minLon, minLat, maxLon, maxLat] = bbox;
  const rows = await allRows("geofences");
  const hit = rows.filter((r) => {
    if (r.min_lat == null || r.max_lat == null || r.min_lng == null || r.max_lng == null) {
      return false;
    }
    const intersects =
      r.min_lng <= maxLon && r.max_lng >= minLon && r.min_lat <= maxLat && r.max_lat >= minLat;
    return intersects && passesFilters(r, filters);
  });
  return collection(hit.map(rowToFeature));
}
