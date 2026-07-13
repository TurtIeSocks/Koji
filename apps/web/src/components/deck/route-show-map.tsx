import type { Layer } from "@deck.gl/core";
import { useEffect, useMemo } from "react";
import { useGetOne, useRecordContext } from "shadmin-core";
import { Button } from "@/components/ui/button";
import { routeCoordsToClusters } from "@/map/lib/calc-request";
import { buildBaseLayers } from "@/map/lib/layers";
import { COLOR } from "@/map/lib/map-colors";
import { geometryBounds } from "./bounds";
import { DeckMap } from "./deck-map";
import { RouteStatsPanel } from "./route-stats-panel";
import { CALC_PERSIST_KEY, useCalc } from "./use-calc";
import { useMarkerOverlay } from "./use-marker-overlay";

interface RouteShowRecord {
	geofence_id?: number | string | null;
	geometry?: GeoJSON.Geometry | null;
	mode?: string;
}

/** Read-only route-show map: the parent geofence outline + the route path +
 *  per-point coverage circles + an optional mode-driven marker overlay + an
 *  auto-computed stats panel. This is `RouteMap` (the edit workbench) with
 *  everything form/calc-write dropped — reads the route from record context
 *  (not react-hook-form) and never calls `calc.run`. No props. */
export function RouteShowMap() {
	const record = useRecordContext<RouteShowRecord>();
	const geofenceId = record?.geofence_id ?? undefined;
	const geometry = record?.geometry ?? null;
	const routeMode = record?.mode;

	// Reactive area: the parent fence, mirrors route-map.tsx:41-56.
	const { data: fence } = useGetOne(
		"geofence",
		{ id: geofenceId! },
		{ enabled: geofenceId != null },
	);
	const fenceFeature = useMemo<GeoJSON.Feature | null>(
		() =>
			fence?.geometry
				? {
						type: "Feature",
						geometry: fence.geometry as GeoJSON.Geometry,
						properties: {},
					}
				: null,
		[fence],
	);

	// Read-only calc settings (radius/minPoints feed the coverage circles and
	// the stats job) — this page never submits a calc.
	const calc = useCalc(undefined, CALC_PERSIST_KEY);
	// Throwaway instance for the auto-computed route-stats job — a separate
	// instance so it can't collide with a "real" calc slot elsewhere.
	const statsCalc = useCalc();

	// Mode-driven marker preview, scoped to the fence polygon. Always fetches
	// (even while the toggle is off) so the stats job has points to work with.
	const overlay = useMarkerOverlay(routeMode, fenceFeature?.geometry, {
		alwaysFetch: true,
	});

	const routeFeature = useMemo<GeoJSON.Feature | null>(
		() => (geometry ? { type: "Feature", geometry, properties: {} } : null),
		[geometry],
	);
	const routeFC = useMemo<GeoJSON.FeatureCollection>(
		() => ({
			type: "FeatureCollection",
			features: routeFeature ? [routeFeature] : [],
		}),
		[routeFeature],
	);

	// The route's ordered centers, and the golbat points it should cover (union
	// of the mode's categories) → a `routeStats` job, mirrors route-map.tsx:135-163.
	const clusters = useMemo(
		() => routeCoordsToClusters(routeFeature),
		[routeFeature],
	);
	const dataPoints = useMemo<[number, number][]>(() => {
		const d = overlay.data;
		return [
			...(d.gym ?? []),
			...(d.pokestop ?? []),
			...(d.spawnpoint ?? []),
			...(d.station ?? []),
		];
	}, [overlay.data]);

	const { runStats } = statsCalc;
	const { radius, minPoints } = calc.params;
	useEffect(() => {
		if (clusters.length === 0 || dataPoints.length === 0) return;
		// Debounced: marker refetches / radius edits shouldn't spam jobs.
		const t = setTimeout(() => {
			void runStats({ dataPoints, clusters, radius, minPoints });
		}, 600);
		return () => clearTimeout(t);
	}, [clusters, dataPoints, radius, minPoints, runStats]);

	const layers = useMemo<Layer[]>(() => {
		const fenceFC: GeoJSON.FeatureCollection = fenceFeature
			? { type: "FeatureCollection", features: [fenceFeature] }
			: { type: "FeatureCollection", features: [] };
		return buildBaseLayers({
			visibility: {
				gyms: overlay.on,
				pokestops: overlay.on,
				spawnpoints: overlay.on,
				stations: overlay.on,
				geofences: true,
				// `routes` off: the route already renders via calcResult below (path +
				// centers) — the plain routes layer would double it.
				routes: false,
				s2: false,
			},
			markerSets: [
				{
					id: "gyms",
					points: overlay.data.gym ?? [],
					color: COLOR.gym,
					radius: 70,
					maxPixels: 12,
				},
				{
					id: "pokestops",
					points: overlay.data.pokestop ?? [],
					color: COLOR.pokestop,
					radius: 40,
					maxPixels: 6,
				},
				{
					id: "spawnpoints",
					points: overlay.data.spawnpoint ?? [],
					color: COLOR.spawnpoint,
					radius: 12,
					maxPixels: 2,
				},
				{
					id: "stations",
					points: overlay.data.station ?? [],
					color: COLOR.station,
					radius: 40,
					maxPixels: 6,
				},
			],
			geofences: fenceFC,
			routes: { type: "FeatureCollection", features: [] },
			s2Cells: [],
			markerRadius: 70,
			onClick: () => {},
			pickable: false,
			calcResult: routeFeature ? routeFC : null,
			calcResultIsRoute: true,
			// Coverage circles at the current radius — ALWAYS shown (unlike RouteMap,
			// which gates this on strategy === "radius"): the show page runs no calc,
			// so there's no "s2 strategy" result to fall back to instead.
			calcResultRadius: calc.params.radius ?? 70,
		});
	}, [fenceFeature, routeFeature, routeFC, overlay.on, overlay.data, calc.params.radius]);

	const fit = geometry
		? geometryBounds(geometry)
		: fenceFeature?.geometry
			? geometryBounds(fenceFeature.geometry)
			: null;

	// DeckMap fits its camera once on mount. The record's geometry is present
	// from the first render (record context, not an async form), so this only
	// matters for a geometry-less route whose fence is still loading — remount
	// (via key) once the fence arrives so the map isn't stuck at [0,0].
	const fitKey = geometry ? "geo" : fenceFeature ? `fence-${geofenceId}` : "empty";

	const panelLoading = statsCalc.stats == null && statsCalc.job != null;

	return (
		<DeckMap
			key={fitKey}
			layers={layers}
			fitBounds={fit}
			height={640}
			expandable
			controller={{ doubleClickZoom: true }}
		>
			{overlay.available ? (
				<Button
					type="button"
					size="sm"
					variant="outline"
					className="absolute left-2 top-2 z-10 bg-background/95"
					onClick={() => overlay.setOn(!overlay.on)}
				>
					{overlay.on ? `Hide ${overlay.label}` : `Show ${overlay.label}`}
				</Button>
			) : null}
			<RouteStatsPanel stats={statsCalc.stats} loading={panelLoading} />
		</DeckMap>
	);
}
