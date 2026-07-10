import type { Layer } from "@deck.gl/core";
import { ArrowLeft, Save } from "lucide-react";
import { useDataProvider, useNotify } from "ra-core";
import { useEffect, useMemo, useRef, useState } from "react";
import {
	FormProvider,
	useForm,
	useFormContext,
	useWatch,
} from "react-hook-form";
import { Link } from "react-router";
import { Button } from "@/components/ui/button";
import { useStartCenter } from "@/lib/use-start-center";
import { useMarkers } from "@/map/data/use-markers";
import { routeCoords } from "@/map/lib/calc-overlay";
import { featureToAreaFC } from "@/map/lib/calc-request";
import { buildBaseLayers } from "@/map/lib/layers";
import type { Bounds, MarkerCategory } from "@/map/stores/types";
import { geometryBounds } from "./bounds";
import { CalcControls } from "./calc-controls";
import { DeckGeoJsonInput } from "./deck-geojson-input";
import { featuresToGeofence, geofenceToFeatures } from "./geofence-geometry";
import { loadCamera, saveCamera } from "./map-camera-storage";
import { routeModeMarkerCategories, routeModeToCategory } from "./route-mode";
import { SaveDialog, type SavePayload } from "./save-dialog";
import { useCalc } from "./use-calc";
import { useGeometryHistory } from "./use-geometry-history";

const WORLD: Bounds = [-180, -85, 180, 85];
const EMPTY_FC: GeoJSON.FeatureCollection = {
	type: "FeatureCollection",
	features: [],
};
const DATA_MODES = [
	{ id: "pokemon", label: "Pokémon" },
	{ id: "quest", label: "Quest" },
	{ id: "fort", label: "Fort" },
];

interface PlaygroundForm {
	geometry: GeoJSON.Geometry | null;
	/** Drives the marker categories + calc golbat category (like a route mode). */
	mode: string;
}

/** Scratch map at `/map`: draw an area with the full geofence tools, pick a data
 *  mode, run calc over the drawing, and visualize — nothing is loaded or saved. */
export function MapPlayground() {
	const methods = useForm<PlaygroundForm>({
		defaultValues: { geometry: null, mode: "pokemon" },
	});
	return (
		<FormProvider {...methods}>
			<PlaygroundBody />
		</FormProvider>
	);
}

function PlaygroundBody() {
	const { setValue } = useFormContext<PlaygroundForm>();
	const geometry = useWatch({ name: "geometry" }) as
		| GeoJSON.Geometry
		| null
		| undefined;
	const mode = useWatch({ name: "mode" }) as string;
	const category = routeModeToCategory(mode);

	const [startLat, startLon] = useStartCenter();
	// Persisted camera wins over the server start center; read once at mount.
	const storedCamera = useMemo(() => loadCamera(), []);
	const defaultViewState = storedCamera ?? {
		longitude: startLon,
		latitude: startLat,
		zoom: 11,
	};
	// DeckMap locks its camera on mount. A stored camera is known synchronously,
	// but the start-center fallback resolves async (useStartCenter). Key the map on
	// the resolved fallback so a cold first load (no stored camera, config still
	// in-flight → [0,0]) remounts once onto the real center instead of sticking at
	// [0,0]. Stable once resolved, so a later pan doesn't remount.
	const camKey = storedCamera ? "cam-stored" : `cam-${startLat}-${startLon}`;

	// Undo/redo over the drawn geometry (playground only).
	const history = useGeometryHistory("geometry");

	// Markers scoped to the drawn area, by data mode. Nothing until something's drawn.
	const area = geometry ?? null;
	const bbox = useMemo<Bounds>(
		() => (geometry ? (geometryBounds(geometry) ?? WORLD) : WORLD),
		[geometry],
	);
	const showCats = routeModeMarkerCategories(mode);
	const want = (c: MarkerCategory) => !!geometry && showCats.includes(c);
	const gyms = useMarkers("gym", area, bbox, 0, want("gym"));
	const stops = useMarkers("pokestop", area, bbox, 0, want("pokestop"));
	const spawns = useMarkers("spawnpoint", area, bbox, 0, want("spawnpoint"));
	const stations = useMarkers("station", area, bbox, 0, want("station"));

	const calc = useCalc();

	const onRun = () => {
		if (!geometry) return;
		void calc.run({
			area: featureToAreaFC({ type: "Feature", geometry, properties: {} }),
			category,
		});
	};

	// Persist the scratch drawing. ALWAYS creates a geofence from the polygon; if
	// the payload includes a route (a calc result exists) AND it has points, also
	// creates that route linked to the geofence (route.geofence_id is required).
	const dataProvider = useDataProvider();
	const notify = useNotify();
	const [saveOpen, setSaveOpen] = useState(false);
	const [busy, setBusy] = useState(false);
	// Idempotency latch: the two creates aren't atomic, so if the route step fails
	// after the geofence was created we keep its id and reuse it on retry (rather
	// than orphan + duplicate the geofence). Cleared when the drawing changes.
	const savedFenceRef = useRef<number | string | null>(null);
	useEffect(() => {
		savedFenceRef.current = null;
	}, [geometry]);

	const onSave = async (payload: SavePayload) => {
		if (!geometry) return;
		const route = payload.route;
		const pts = route && calc.result ? routeCoords(calc.result) : null;
		const routeReady = !!route && !!pts && pts.length > 0;
		const { name, mode: saveMode, parent } = payload.geofence;
		setBusy(true);
		try {
			// Reuse a geofence already created earlier in this save session; else create it.
			let fenceId = savedFenceRef.current;
			if (fenceId == null) {
				const { data: fence } = await dataProvider.create("geofence", {
					data: { name, mode: saveMode, parent, geometry },
				});
				fenceId = fence.id;
				savedFenceRef.current = fenceId;
			}
			if (routeReady && route && pts) {
				await dataProvider.create("route", {
					data: {
						name: route.name,
						mode: saveMode,
						geofence_id: fenceId,
						geometry: { type: "MultiPoint", coordinates: pts },
					},
				});
				notify(`Saved geofence "${name}" + route "${route.name}"`, {
					type: "success",
				});
			} else if (route) {
				// Route requested but the calc produced no points — geofence still saved.
				notify(`Saved geofence "${name}" — calc had no route points, route skipped`, {
					type: "warning",
				});
			} else {
				notify(`Saved geofence "${name}"`, { type: "success" });
			}
			savedFenceRef.current = null;
			setSaveOpen(false);
		} catch (e) {
			notify(`Save failed: ${e instanceof Error ? e.message : String(e)}`, {
				type: "error",
			});
		} finally {
			setBusy(false);
		}
	};

	const contextLayers = useMemo<Layer[]>(() => {
		const cats = routeModeMarkerCategories(mode);
		const on = (c: MarkerCategory) => !!geometry && cats.includes(c);
		return buildBaseLayers({
			visibility: {
				gyms: on("gym"),
				pokestops: on("pokestop"),
				spawnpoints: on("spawnpoint"),
				stations: on("station"),
				geofences: false,
				routes: false,
				s2: false,
			},
			markerSets: [
				{
					id: "gyms",
					points: gyms.data ?? [],
					color: [230, 80, 80],
					radius: 70,
					maxPixels: 12,
				},
				{
					id: "pokestops",
					points: stops.data ?? [],
					color: [0, 120, 255],
					radius: 40,
					maxPixels: 6,
				},
				{
					id: "spawnpoints",
					points: spawns.data ?? [],
					color: [40, 200, 120],
					radius: 12,
					maxPixels: 2,
				},
				{
					id: "stations",
					points: stations.data ?? [],
					color: [150, 80, 220],
					radius: 40,
					maxPixels: 6,
				},
			],
			geofences: EMPTY_FC,
			routes: EMPTY_FC,
			s2Cells: [],
			markerRadius: 70,
			onClick: () => {},
			pickable: false,
			calcResult: calc.result,
			calcResultIsRoute: true,
			calcResultRadius:
				calc.params.strategy === "radius" ? calc.params.radius : undefined,
		});
	}, [
		geometry,
		mode,
		gyms.data,
		stops.data,
		spawns.data,
		stations.data,
		calc.result,
		calc.params.radius,
		calc.params.strategy,
	]);

	const overlay = (
		<>
			{/* Dock stops above the bottom draw toolbar so the two don't overlap at the
          lower-left corner (the toolbar spans full width beneath it). */}
			<div className="absolute bottom-11 left-0 top-0 z-10">
				<CalcControls
					calc={calc}
					category={category}
					onRun={onRun}
					disabled={!geometry}
					disabledReason={!geometry ? "Draw an area first" : undefined}
					header={
						<Button
							asChild
							size="sm"
							variant="ghost"
							className="-mx-1 justify-start"
						>
							<Link to="/">
								<ArrowLeft className="size-4" /> Admin
							</Link>
						</Button>
					}
					footer={
						<Button
							type="button"
							size="sm"
							className="w-full"
							disabled={!geometry}
							onClick={() => setSaveOpen(true)}
						>
							<Save className="size-4" /> Save
						</Button>
					}
				/>
			</div>
			<div className="absolute top-2 right-2 z-10 flex items-center gap-2">
				<div className="flex gap-1 rounded-md border bg-background/95 p-1 shadow-sm backdrop-blur">
					{DATA_MODES.map((m) => (
						<Button
							key={m.id}
							type="button"
							size="sm"
							variant={mode === m.id ? "default" : "ghost"}
							onClick={() => setValue("mode", m.id)}
						>
							{m.label}
						</Button>
					))}
				</div>
			</div>
		</>
	);

	return (
		<>
			<DeckGeoJsonInput
				key={camKey}
				source="geometry"
				height="100dvh"
				contextLayers={contextLayers}
				toFeatures={geofenceToFeatures}
				fromFeatures={featuresToGeofence}
				defaultViewState={defaultViewState}
				overlay={overlay}
				history={history}
				onViewStateChange={(vs) =>
					saveCamera({
						longitude: vs.longitude,
						latitude: vs.latitude,
						zoom: vs.zoom,
					})
				}
			/>
			<SaveDialog
				open={saveOpen}
				onOpenChange={setSaveOpen}
				hasRoute={!!calc.result}
				defaultMode={mode}
				busy={busy}
				onConfirm={onSave}
			/>
		</>
	);
}
