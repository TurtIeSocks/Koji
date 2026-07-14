import type { Layer } from "@deck.gl/core";
import { useEffect, useMemo } from "react";
import { useFormContext, useWatch } from "react-hook-form";
import { useGetOne } from "shadmin-core";
import { useMarkers } from "@/map/data/use-markers";
import { routeCoords } from "@/map/lib/calc-overlay";
import { featureToAreaFC, routeCoordsToClusters } from "@/map/lib/calc-request";
import { buildBaseLayers } from "@/map/lib/layers";
import { COLOR } from "@/map/lib/map-colors";
import type { Bounds, MarkerCategory } from "@/map/stores/types";
import { geometryBounds } from "./bounds";
import { CalcControls } from "./calc-controls";
import { DeckMap } from "./deck-map";
import { LastSeenPicker } from "./last-seen-picker";
import { routeModeMarkerCategories, routeModeToCategory } from "./route-mode";
import { RouteStatsPanel } from "./route-stats-panel";
import { CALC_PERSIST_KEY, useCalc } from "./use-calc";
import { useLastSeen } from "./use-last-seen";

const WORLD: Bounds = [-180, -85, 180, 85];

/** Route-edit calc workbench: reactively loads the parent geofence's area
 *  (`geofence_id`), runs a cluster/bootstrap calc over it, and writes the
 *  ordered result into the route's `geometry`. The golbat category comes from
 *  the route's `mode` (not a panel dropdown). Reads/writes the surrounding
 *  route form via react-hook-form context; `height` threads to its DeckMap
 *  (defaults to the original hardcoded 640 so bare usages are unchanged). */
export function RouteMap({ height = 640 }: { height?: number | string }) {
	const form = useFormContext();
	const geofenceId = useWatch({ name: "geofence_id" }) as
		| number
		| string
		| undefined;
	const geometry = useWatch({ name: "geometry" }) as
		| GeoJSON.Geometry
		| null
		| undefined;
	const routeMode = useWatch({ name: "mode" }) as string | undefined;
	const category = routeModeToCategory(routeMode);

	// Reactive area: the parent fence, re-fetched when geofence_id changes.
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

	// Golbat markers to preview for the route's mode, scoped to the fence polygon.
	// Nothing shows until a geofence is selected (→ the fence loads): `want` gates
	// every fetch on fenceFeature.
	const markerArea = fenceFeature?.geometry ?? null;
	const markerBbox = useMemo<Bounds>(
		() =>
			fenceFeature?.geometry
				? (geometryBounds(fenceFeature.geometry) ?? WORLD)
				: WORLD,
		[fenceFeature],
	);
	// "Last seen after" filter (epoch seconds); 0 = show all.
	const lastSeen = useLastSeen();

	// Calc settings persist across route edits (shared global key, like last seen).
	// Declared before the markers so the spawnpoint fetch can honor its tth.
	const calc = useCalc(undefined, CALC_PERSIST_KEY);
	// Separate instance so the loaded-route stats job never disturbs the main
	// calc's result (which writes back into the route geometry). Not persisted —
	// its params are throwaway (only radius/minPoints from `calc` feed stats).
	const statsCalc = useCalc();

	// Tth only applies to a spawnpoint cluster calc (bootstrap ignores it). Gates
	// both the Tth dropdown and the spawnpoint preview filter so the preview
	// matches what Calculate will do.
	const showTth = calc.params.mode === "cluster" && category === "spawnpoint";

	const showCats = routeModeMarkerCategories(routeMode);
	const want = (c: MarkerCategory) => !!fenceFeature && showCats.includes(c);
	const gyms = useMarkers(
		"gym",
		markerArea,
		markerBbox,
		lastSeen.epoch,
		want("gym"),
	);
	const stops = useMarkers(
		"pokestop",
		markerArea,
		markerBbox,
		lastSeen.epoch,
		want("pokestop"),
	);
	// tth rides along so the spawnpoint preview AND the derived routeStats
	// dataPoints honor the confirmed/unconfirmed filter — otherwise changing tth
	// wouldn't refetch and the stats would go stale.
	const spawns = useMarkers(
		"spawnpoint",
		markerArea,
		markerBbox,
		lastSeen.epoch,
		want("spawnpoint"),
		showTth ? calc.params.tth : undefined,
	);
	const stations = useMarkers(
		"station",
		markerArea,
		markerBbox,
		lastSeen.epoch,
		want("station"),
	);

	// A succeeded calc result → the route's geometry (MultiPoint of ordered points).
	useEffect(() => {
		if (!calc.result) return;
		const pts = routeCoords(calc.result); // [lon,lat][]
		if (pts.length === 0) return;
		form.setValue(
			"geometry",
			{ type: "MultiPoint", coordinates: pts },
			{ shouldDirty: true },
		);
	}, [calc.result, form]);

	// The loaded/edited route as ordered [lat,lon] centers, and the golbat points it
	// should cover (union of the mode's categories) → a `routeStats` job so an
	// existing route reports coverage/score/distance without re-clustering.
	const clusters = useMemo(
		() =>
			routeCoordsToClusters(
				geometry ? { type: "Feature", geometry, properties: {} } : null,
			),
		[geometry],
	);
	const dataPoints = useMemo<[number, number][]>(() => {
		const byCat: Record<string, [number, number][]> = {
			gym: gyms.data ?? [],
			pokestop: stops.data ?? [],
			spawnpoint: spawns.data ?? [],
			station: stations.data ?? [],
		};
		return showCats.flatMap((c) => byCat[c] ?? []);
	}, [gyms.data, stops.data, spawns.data, stations.data, showCats]);

	const { runStats } = statsCalc;
	const { radius, minPoints } = calc.params;
	useEffect(() => {
		// A live calc supplies its own stats; only auto-compute for a loaded route.
		if (calc.result) return;
		if (clusters.length === 0 || dataPoints.length === 0) return;
		// Debounced: radius/minPoints edits and marker refetches shouldn't spam jobs.
		const t = setTimeout(() => {
			void runStats({ dataPoints, clusters, radius, minPoints });
		}, 600);
		return () => clearTimeout(t);
	}, [clusters, dataPoints, calc.result, radius, minPoints, runStats]);

	const onRun = () => {
		if (!fenceFeature) return;
		// lastSeen rides along so the server clusters the same fresh points the
		// marker preview shows (0 = no filter).
		void calc.run({
			area: featureToAreaFC(fenceFeature),
			category,
			lastSeen: lastSeen.epoch,
		});
	};

	// Floating-panel stats: a live calc's stats win, else the loaded route's.
	const panelStats = calc.stats ?? statsCalc.stats;
	const panelLoading =
		panelStats == null && (calc.job != null || statsCalc.job != null);

	const layers = useMemo<Layer[]>(() => {
		const cats = routeModeMarkerCategories(routeMode);
		const on = (c: MarkerCategory) => !!fenceFeature && cats.includes(c);
		const fenceFC: GeoJSON.FeatureCollection = fenceFeature
			? { type: "FeatureCollection", features: [fenceFeature] }
			: { type: "FeatureCollection", features: [] };
		const routeFC: GeoJSON.FeatureCollection = geometry
			? {
					type: "FeatureCollection",
					features: [{ type: "Feature", geometry, properties: {} }],
				}
			: { type: "FeatureCollection", features: [] };
		// The active calc result OR (when just editing) the loaded route — rendered
		// the SAME way: ordered centers, coverage circles at the current radius, and
		// the distance-colored path. So an edited route shows its cluster circles too,
		// not only the centers.
		const displayResult = calc.result ?? (geometry ? routeFC : null);
		return buildBaseLayers({
			// `routes` off: displayResult already draws the route (path + centers), so
			// the plain routes layer would double it.
			visibility: {
				gyms: on("gym"),
				pokestops: on("pokestop"),
				spawnpoints: on("spawnpoint"),
				stations: on("station"),
				geofences: true,
				routes: false,
				s2: false,
			},
			markerSets: [
				{
					id: "gyms",
					points: gyms.data ?? [],
					color: COLOR.gym,
					radius: 70,
					maxPixels: 12,
				},
				{
					id: "pokestops",
					points: stops.data ?? [],
					color: COLOR.pokestop,
					radius: 40,
					maxPixels: 6,
				},
				{
					id: "spawnpoints",
					points: spawns.data ?? [],
					color: COLOR.spawnpoint,
					radius: 12,
					maxPixels: 2,
				},
				{
					id: "stations",
					points: stations.data ?? [],
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
			calcResult: displayResult,
			calcResultIsRoute: true,
			// Coverage circles at the exact calc radius (radius strategy only) — updates
			// live as the user drags the radius, for both a calc result and a loaded route.
			calcResultRadius:
				calc.params.strategy === "radius" ? calc.params.radius : undefined,
		});
	}, [
		fenceFeature,
		geometry,
		calc.result,
		gyms.data,
		stops.data,
		spawns.data,
		stations.data,
		routeMode,
		calc.params.radius,
		calc.params.strategy,
	]);

	const fit = geometry
		? geometryBounds(geometry)
		: fenceFeature?.geometry
			? geometryBounds(fenceFeature.geometry)
			: null;
	const areaMissing = !fenceFeature;

	// DeckMap fits its camera once on mount. On CREATE the form starts empty, so
	// remount (via key) when the framing target first appears — a picked fence,
	// then the calc result — so the map isn't stuck at [0,0]. On EDIT geometry is
	// present from the first render, so the key is stable ("geo") → no remount.
	const fitKey = geometry
		? "geo"
		: fenceFeature
			? `fence-${geofenceId}`
			: "empty";

	return (
		<div className="relative">
			<DeckMap
				key={fitKey}
				layers={layers}
				fitBounds={fit}
				height={height}
				controller={{ doubleClickZoom: true }}
			>
				<div className="absolute inset-y-0 left-0 z-10">
					<CalcControls
						calc={calc}
						onRun={onRun}
						disabled={areaMissing}
						disabledReason={
							areaMissing ? "Select a geofence first." : undefined
						}
					/>
				</div>
				<div className="absolute right-2 top-2 z-10">
					<LastSeenPicker
						value={lastSeen.value}
						onChange={lastSeen.setValue}
						tth={showTth ? calc.params.tth : undefined}
						onTthChange={
							showTth ? (t) => calc.setParams({ tth: t }) : undefined
						}
					/>
				</div>
				<RouteStatsPanel stats={panelStats} loading={panelLoading} />
			</DeckMap>
		</div>
	);
}
