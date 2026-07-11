import type { Layer } from "@deck.gl/core";
import { useMemo, useState } from "react";
import { useWatch } from "react-hook-form";
import { Button } from "@/components/ui/button";
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
import { useLastSeen } from "./use-last-seen";

const WORLD: Bounds = [-180, -85, 180, 85];

export function GeofenceMap() {
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

	// Empty (create) start view: the server's configured center, not [0,0].
	const [startLat, startLon] = useStartCenter();

	const contextLayers = useMemo<Layer[]>(
		() =>
			buildBaseLayers({
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
		[show, gyms.data, stops.data, spawns.data, s2.data],
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
				<LastSeenPicker
					value={lastSeen.value}
					onChange={lastSeen.setValue}
					className="ml-auto"
				/>
			</div>
			<DeckGeoJsonInput
				source="geometry"
				label="Geometry"
				height={480}
				contextLayers={contextLayers}
				toFeatures={geofenceToFeatures}
				fromFeatures={featuresToGeofence}
				defaultViewState={{ longitude: startLon, latitude: startLat, zoom: 10 }}
			/>
		</div>
	);
}
