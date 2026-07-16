import type { Bounds } from "@/map/stores/types";
import { geometryBounds } from "./bounds";
import { padBbox } from "./use-neighbor-overlay";

/** Below this zoom the neighbour fetch is suppressed — the "honest cap"
 *  against fetching the whole prod geofence table (~600 fences, uncapped
 *  endpoint) whenever the camera is zoomed out. */
export const NEIGHBOR_MIN_ZOOM = 9;

/** Round to 4dp (~11m) so pan jitter doesn't produce endless new query keys. */
export function roundBbox(b: Bounds): Bounds {
	return b.map((n) => Math.round(n * 10_000) / 10_000) as unknown as Bounds;
}

/** Fixed-size fallback box around a point (used before the first camera event). */
export function centerBbox(lon: number, lat: number): Bounds {
	return [lon - 0.15, lat - 0.1, lon + 0.15, lat + 0.1];
}

/** The neighbour-fetch bbox for the current situation, or null (= no fetch).
 *  Camera wins once the user has interacted; below the zoom floor the fetch
 *  is suppressed (the "honest cap" — prod has ~600 fences and the endpoint
 *  is uncapped); before any interaction fall back to the drawn geometry,
 *  else a box around the server start-center so the toggle is never a no-op. */
export function neighborBbox(
	view: { bounds: Bounds; zoom: number } | null,
	geometry: GeoJSON.Geometry | null | undefined,
	startLon: number,
	startLat: number,
): Bounds | null {
	if (view) return view.zoom >= NEIGHBOR_MIN_ZOOM ? view.bounds : null;
	return geometry ? padBbox(geometryBounds(geometry), 0.2) : centerBbox(startLon, startLat);
}
