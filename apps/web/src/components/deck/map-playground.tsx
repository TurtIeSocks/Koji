import type { Layer } from "@deck.gl/core";
import { ArrowLeft, Save } from "lucide-react";
import { useDataProvider, useNotify } from "ra-core";
import { useMemo, useState } from "react";
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
import { routeModeMarkerCategories, routeModeToCategory } from "./route-mode";
import { SaveDialog } from "./save-dialog";
import { useCalc } from "./use-calc";

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

	// Persist the scratch drawing. Geofence: just the polygon. Route: create the
	// geofence from the drawing AND the calc result as a route linked to it (a
	// route's geofence_id is required), in one action.
	const dataProvider = useDataProvider();
	const notify = useNotify();
	const [saveKind, setSaveKind] = useState<"geofence" | "route" | null>(null);
	const [busy, setBusy] = useState(false);

	const onSave = async (name: string, saveMode: string) => {
		if (!geometry) return;
		const isRoute = saveKind === "route";
		const pts = isRoute && calc.result ? routeCoords(calc.result) : null;
		if (isRoute && (!pts || pts.length === 0)) {
			notify("Run a calculation first", { type: "warning" });
			return;
		}
		setBusy(true);
		try {
			const { data: fence } = await dataProvider.create("geofence", {
				data: { name, mode: saveMode, geometry },
			});
			if (isRoute && pts) {
				await dataProvider.create("route", {
					data: {
						name,
						mode: saveMode,
						geofence_id: fence.id,
						geometry: { type: "MultiPoint", coordinates: pts },
					},
				});
				notify(`Saved route "${name}" + its area`, { type: "success" });
			} else {
				notify(`Saved geofence "${name}"`, { type: "success" });
			}
			setSaveKind(null);
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
					footer={
						<Button
							type="button"
							size="sm"
							className="w-full"
							disabled={!calc.result}
							onClick={() => setSaveKind("route")}
						>
							<Save className="size-4" /> Save route
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
				<Button
					asChild
					size="sm"
					variant="secondary"
					className="bg-background/95 shadow-sm backdrop-blur"
				>
					<Link to="/">
						<ArrowLeft className="size-4" /> Admin
					</Link>
				</Button>
			</div>
		</>
	);

	return (
		<>
			<DeckGeoJsonInput
				source="geometry"
				height="100dvh"
				contextLayers={contextLayers}
				toFeatures={geofenceToFeatures}
				fromFeatures={featuresToGeofence}
				defaultViewState={{ longitude: startLon, latitude: startLat, zoom: 11 }}
				overlay={overlay}
				toolbarExtra={
					geometry ? (
						<Button
							type="button"
							size="sm"
							variant="secondary"
							onClick={() => setSaveKind("geofence")}
						>
							<Save className="size-4" /> Save geofence
						</Button>
					) : undefined
				}
			/>
			<SaveDialog
				open={saveKind != null}
				onOpenChange={(o) => {
					if (!o) setSaveKind(null);
				}}
				title={saveKind === "route" ? "Save route + area" : "Save geofence"}
				description={
					saveKind === "route"
						? "Saves the drawn area as a geofence and the calc result as a route linked to it."
						: "Saves the drawn area as a new geofence."
				}
				defaultMode={mode}
				busy={busy}
				onConfirm={onSave}
			/>
		</>
	);
}
