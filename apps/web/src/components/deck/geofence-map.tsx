import type { Layer } from "@deck.gl/core";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useWatch } from "react-hook-form";
import { useRecordContext } from "shadmin-core";
import { Button } from "@/components/ui/button";
import { GEOFENCE_MODES } from "@/lib/constants";
import { useMarkers } from "@/map/data/use-markers";
import { useS2Cells } from "@/map/data/use-s2-cells";
import { buildBaseLayers } from "@/map/lib/layers";
import { COLOR } from "@/map/lib/map-colors";
import type { Bounds } from "@/map/stores/types";
import { useStartCenter } from "@/lib/use-start-center";
import { geometryBounds } from "./bounds";
import { DeckGeoJsonInput } from "./deck-geojson-input";
import { featuresToGeofence, geofenceToFeatures } from "./geofence-geometry";
import { LastSeenPicker } from "./last-seen-picker";
import { NEIGHBOR_MIN_ZOOM, neighborBbox, roundBbox } from "./neighbor-bbox";
import { useLastSeen } from "./use-last-seen";
import { useNeighborOverlay } from "./use-neighbor-overlay";

const WORLD: Bounds = [-180, -85, 180, 85];

export function GeofenceMap({ height = 480 }: { height?: number | string }) {
	// Live geometry from the form drives the marker query as the fence is edited.
	// Markers use the actual polygon `area` (so a MultiPolygon returns points
	// inside its parts, not the whole bbox); S2 cells stay bbox-based (a grid).
	const geometry = useWatch({ name: "geometry" }) as
		| GeoJSON.Geometry
		| null
		| undefined;
	const area = geometry ?? null;
	const bbox = useMemo<Bounds>(
		() => (geometry ? (geometryBounds(geometry) ?? WORLD) : WORLD),
		[geometry],
	);

	// Empty (create) start view: the server's configured center, not [0,0].
	// Moved above the neighbour block below — it's the pre-interaction fallback
	// center, not just the initial camera.
	const [startLat, startLon] = useStartCenter();

	// --- Neighbour overlay: camera-driven bbox (spec §4.6) ------------------
	// deck fires onViewStateChange only on interaction, so `view` is null until
	// the user pans/zooms; the fallback keeps the toggle from being a no-op.
	const [view, setView] = useState<{ bounds: Bounds; zoom: number } | null>(null);
	const viewTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
	const onViewStateChange = useCallback((vs: { zoom: number }, bounds: Bounds) => {
		if (viewTimer.current) clearTimeout(viewTimer.current);
		viewTimer.current = setTimeout(
			() => setView({ bounds: roundBbox(bounds), zoom: vs.zoom }),
			400,
		);
	}, []);
	useEffect(() => () => { if (viewTimer.current) clearTimeout(viewTimer.current); }, []);

	const zoomedOut = view != null && view.zoom < NEIGHBOR_MIN_ZOOM;
	const nbBbox = neighborBbox(view, geometry, startLon, startLat);

	// --- Neighbour filters (D4): follow the form until the user overrides ---
	const formMode = useWatch({ name: "mode" }) as string | undefined;
	const formProjects = (useWatch({ name: "projects" }) as number[] | undefined) ?? [];
	const [modeOverride, setModeOverride] = useState<string | null>(null); // null = follow form
	const [onlyMyProjects, setOnlyMyProjects] = useState(true);
	const effectiveMode = modeOverride ?? (formMode && formMode !== "unset" ? formMode : "all");
	const nbFilters = {
		mode: effectiveMode === "all" ? undefined : effectiveMode,
		projects: onlyMyProjects && formProjects.length > 0 ? formProjects : undefined,
	};

	// On edit, exclude the fence being edited from its own "Show Neighbors"
	// overlay; on create there's no id yet, so nothing is excluded.
	const editingId = useRecordContext()?.id;
	const nb = useNeighborOverlay(nbBbox, editingId, nbFilters);

	const [show, setShow] = useState({
		gyms: false,
		pokestops: false,
		spawnpoints: false,
		s2: false,
	});
	// "Last seen after" filter (epoch seconds) — only points updated since the
	// picked datetime; 0 = show all.
	const lastSeen = useLastSeen();
	// Gate data fetches on having a geometry — on create there's no fence yet, so
	// area/bbox would be the whole world (a catastrophic golbat query).
	const hasGeom = !!geometry;
	const gyms = useMarkers("gym", area, bbox, lastSeen.epoch, show.gyms && hasGeom);
	const stops = useMarkers("pokestop", area, bbox, lastSeen.epoch, show.pokestops && hasGeom);
	const spawns = useMarkers("spawnpoint", area, bbox, lastSeen.epoch, show.spawnpoints && hasGeom);
	const s2 = useS2Cells(15, bbox, show.s2 && hasGeom);

	const contextLayers = useMemo<Layer[]>(
		() => [
			...buildBaseLayers({
				visibility: {
					gyms: show.gyms,
					pokestops: show.pokestops,
					spawnpoints: show.spawnpoints,
					stations: false,
					geofences: false,
					routes: false,
					s2: show.s2,
				},
				// Per-category radius (meters, GPU-scaled by zoom) + a pixel cap so dense
				// markers stop overlapping when zoomed in. Spawnpoints are densest → the
				// smallest cap; gyms the largest. Tune here.
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
				],
				geofences: { type: "FeatureCollection", features: [] },
				routes: { type: "FeatureCollection", features: [] },
				s2Cells: s2.data ?? [],
				markerRadius: 70,
				onClick: () => {},
				pickable: false,
			}),
			...nb.layers,
		],
		[show, gyms.data, stops.data, spawns.data, s2.data, nb.layers],
	);

	return (
		<div className="flex flex-col gap-2">
			<div className="flex flex-wrap items-center gap-1">
				{(["gyms", "pokestops", "spawnpoints", "s2"] as const).map((k) => (
					<Button
						key={k}
						type="button"
						size="sm"
						variant={show[k] ? "default" : "secondary"}
						onClick={() => setShow((s) => ({ ...s, [k]: !s[k] }))}
						className="capitalize"
					>
						{k}
					</Button>
				))}
				<Button
					type="button"
					size="sm"
					variant={nb.on ? "default" : "secondary"}
					onClick={() => nb.setOn(!nb.on)}
				>
					{nb.on ? "Hide Neighbors" : "Show Neighbors"}
				</Button>
				{/* Filter controls stay visible (not gated on nb.on) so the user can
				    dial in mode/project scope before ever flipping the toggle on —
				    matches how effectiveMode/nbFilters are already computed every
				    render regardless of `on`. Only the zoom hint is contextual. */}
				<select
					aria-label="Neighbor mode filter"
					value={effectiveMode}
					onChange={(e) => setModeOverride(e.target.value)}
					className="rounded-md border bg-background px-2 py-1 text-sm"
				>
					<option value="all">All modes</option>
					{GEOFENCE_MODES.map((m) => (
						<option key={m.id} value={m.id}>
							{m.name}
						</option>
					))}
				</select>
				<Button
					type="button"
					size="sm"
					aria-label="My projects only"
					variant={onlyMyProjects && formProjects.length > 0 ? "default" : "secondary"}
					disabled={formProjects.length === 0}
					title={
						formProjects.length === 0
							? "Set projects on the Details tab to scope neighbors"
							: undefined
					}
					onClick={() => setOnlyMyProjects((v) => !v)}
				>
					My projects
				</Button>
				{nb.on && zoomedOut ? (
					<span className="text-xs text-muted-foreground">
						Zoom in to load neighbors
					</span>
				) : null}
				<LastSeenPicker
					value={lastSeen.value}
					onChange={lastSeen.setValue}
					className="ml-auto"
				/>
			</div>
			<DeckGeoJsonInput
				source="geometry"
				label="Geometry"
				height={height}
				contextLayers={contextLayers}
				getTooltip={nb.getTooltip}
				toFeatures={geofenceToFeatures}
				fromFeatures={featuresToGeofence}
				defaultViewState={{ longitude: startLon, latitude: startLat, zoom: 10 }}
				onViewStateChange={onViewStateChange}
			/>
		</div>
	);
}
