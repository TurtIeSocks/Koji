import type { Layer } from "@deck.gl/core";
import { ScatterplotLayer } from "@deck.gl/layers";
import { useMemo, useState } from "react";
import { useMarkers } from "@/map/data/use-markers";
import { packMarkers } from "@/map/lib/coords";
import { COLOR } from "@/map/lib/map-colors";
import type { Bounds, MarkerCategory } from "@/map/stores/types";
import { geometryBounds } from "./bounds";
import { routeModeMarkerCategories } from "./route-mode";

const WORLD: Bounds = [-180, -85, 180, 85];

interface MarkerStyle {
	color: readonly [number, number, number];
	radius: number;
	maxPixels: number;
}

// Mirrors RouteMap's markerSets (src/components/deck/route-map.tsx:210-243).
// "fort" is a route MODE, never a golbat MarkerCategory itself (fort mode
// expands to gym/station/pokestop) — its entry here is unused but keeps this
// a total Record<MarkerCategory, ...> instead of a partial lookup.
const STYLE: Record<MarkerCategory, MarkerStyle> = {
	gym: { color: COLOR.gym, radius: 70, maxPixels: 12 },
	pokestop: { color: COLOR.pokestop, radius: 40, maxPixels: 6 },
	spawnpoint: { color: COLOR.spawnpoint, radius: 12, maxPixels: 2 },
	station: { color: COLOR.station, radius: 40, maxPixels: 6 },
	fort: { color: COLOR.gym, radius: 70, maxPixels: 12 },
};

const LABELS: Record<string, string> = {
	pokemon: "Spawnpoints",
	quest: "Pokestops",
	fort: "Forts",
};

export interface UseMarkerOverlayOpts {
	/** route-show passes true so a stats job can read `data` while the toggle
	 *  is off — the fetch runs regardless of `on`. */
	alwaysFetch?: boolean;
}

export interface UseMarkerOverlayResult {
	on: boolean;
	setOn: (v: boolean) => void;
	/** Human label for the toggle button, e.g. "Spawnpoints" / "Forts". */
	label: string;
	/** Built only while `on` — spread into the consumer's layer array. */
	markerLayers: Layer[];
	/** Per-category points for the mode's categories, regardless of `on`
	 *  (when fetched — see `alwaysFetch`). For a route-show stats job. */
	data: Partial<Record<MarkerCategory, [number, number][]>>;
}

/** Mode-driven "show markers" toggle for a read-only route/geofence show page.
 *  Fetches the golbat categories implied by `mode` (`routeModeMarkerCategories`)
 *  scoped to `area`, and renders them as deck ScatterplotLayers. Default OFF.
 *
 *  Calls `useMarkers` FOUR times, UNCONDITIONALLY, every render (fixed hook
 *  order per the rules of hooks) — each individual fetch is gated via its own
 *  `enabled` arg instead of being skipped. `area` may be null/undefined (e.g.
 *  while a parent record is still loading); the hooks still run, just scoped
 *  to a WORLD bbox with everything disabled. */
export function useMarkerOverlay(
	mode: string | undefined,
	area: GeoJSON.Geometry | null | undefined,
	opts?: UseMarkerOverlayOpts,
): UseMarkerOverlayResult {
	const [on, setOn] = useState(false);

	// Stable per `mode` string — routeModeMarkerCategories returns the SAME
	// array reference from its module-level lookup table, so this is safe to
	// use directly as a memo dependency below.
	const cats = routeModeMarkerCategories(mode);
	const areaArg = area ?? null;
	const bbox: Bounds = area ? (geometryBounds(area) ?? WORLD) : WORLD;

	// No area yet (parent record still loading) → every fetch stays disabled,
	// even with `alwaysFetch`/`on` set — a WORLD-bbox query with no polygon
	// scope would be an unbounded planet-wide fetch.
	const wantFetch = (c: MarkerCategory) =>
		!!area && (opts?.alwaysFetch || on) && cats.includes(c);

	const gyms = useMarkers("gym", areaArg, bbox, 0, wantFetch("gym"));
	const stops = useMarkers("pokestop", areaArg, bbox, 0, wantFetch("pokestop"));
	const spawns = useMarkers(
		"spawnpoint",
		areaArg,
		bbox,
		0,
		wantFetch("spawnpoint"),
	);
	const stations = useMarkers(
		"station",
		areaArg,
		bbox,
		0,
		wantFetch("station"),
	);

	const pointsByCategory: Record<MarkerCategory, [number, number][] | undefined> = {
		gym: gyms.data,
		pokestop: stops.data,
		spawnpoint: spawns.data,
		station: stations.data,
		fort: undefined,
	};

	const data = useMemo<Partial<Record<MarkerCategory, [number, number][]>>>(() => {
		const out: Partial<Record<MarkerCategory, [number, number][]>> = {};
		for (const c of cats) {
			const pts = pointsByCategory[c];
			if (pts) out[c] = pts;
		}
		return out;
		// eslint-disable-next-line react-hooks/exhaustive-deps
	}, [cats, gyms.data, stops.data, spawns.data, stations.data]);

	const markerLayers = useMemo<Layer[]>(() => {
		if (!on) return [];
		const layers: Layer[] = [];
		for (const c of cats) {
			const pts = data[c];
			if (!pts || pts.length === 0) continue;
			const style = STYLE[c];
			layers.push(
				new ScatterplotLayer({
					id: `markers-${c}`,
					data: {
						length: pts.length,
						attributes: { getPosition: { value: packMarkers(pts), size: 2 } },
					},
					// Radius in METERS → the GPU scales it with zoom for free.
					getRadius: style.radius,
					radiusUnits: "meters",
					radiusMinPixels: 1,
					radiusMaxPixels: style.maxPixels,
					getFillColor: [...style.color, 200],
				}),
			);
		}
		return layers;
	}, [on, cats, data]);

	const label = LABELS[mode ?? ""] ?? "Markers";

	return { on, setOn, label, markerLayers, data };
}
