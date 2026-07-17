import type { Bounds, MarkerCategory } from "@/map/stores/types";
import type { TthFilter } from "../types";
import { allRows } from "./db";
import { generateMarkers, type MarkerStore } from "./seeds/markers";
import { ensureSeeded } from "./seeds/seed";

// The synthetic marker world is deterministic in the seeded geofence
// geometries (see seeds/markers.ts) and expensive to build (~24k points), so
// generate it once per session and serve every query from the cached store.
let storePromise: Promise<MarkerStore> | null = null;

async function getMarkerStore(): Promise<MarkerStore> {
  if (!storePromise) {
    storePromise = (async () => {
      await ensureSeeded();
      const rows = await allRows("geofences");
      const fc: GeoJSON.FeatureCollection = {
        type: "FeatureCollection",
        features: rows.map((r) => ({
          type: "Feature",
          geometry: r.geometry,
          properties: { id: r.id, name: r.name, mode: r.mode },
        })),
      };
      return generateMarkers(fc);
    })();
  }
  return storePromise;
}

/** Demo mirror of the live `POST /golbat-data/{category}` marker fetch: adapt
 *  the (area | bounds) request into the in-memory `MarkerStore.query` opts.
 *  Prefers the real polygon `area` over the bounding box, exactly like live
 *  (a bbox would wrongly include points in a MultiPolygon's gaps). Returns
 *  Koji `[lat, lon]` pairs. */
export async function fetchMarkers(
  category: MarkerCategory,
  area: GeoJSON.Geometry | null,
  bounds: Bounds,
  lastSeen: number,
  tth?: TthFilter,
): Promise<[number, number][]> {
  const store = await getMarkerStore();
  const [minLon, minLat, maxLon, maxLat] = bounds;
  const opts = area
    ? { areaFeatures: [{ type: "Feature" as const, geometry: area, properties: {} }], lastSeen, tth }
    : { bbox: { minLat, minLon, maxLat, maxLon }, lastSeen, tth };
  return store.query(category, opts);
}
