import { useEffect, useMemo } from "react";
import { useFormContext, useWatch } from "react-hook-form";
import { useGetOne } from "shadmin-core";
import type { Layer } from "@deck.gl/core";
import { buildBaseLayers, routePathLayer } from "@/map/lib/layers";
import { featureToAreaFC } from "@/map/lib/calc-request";
import { routeCoords } from "@/map/lib/calc-overlay";
import { useMarkers } from "@/map/data/use-markers";
import type { Bounds, MarkerCategory } from "@/map/stores/types";
import { DeckMap } from "./deck-map";
import { geometryBounds } from "./bounds";
import { useCalc } from "./use-calc";
import { CalcControls } from "./calc-controls";
import { routeModeToCategory, routeModeMarkerCategories } from "./route-mode";

const WORLD: Bounds = [-180, -85, 180, 85];

/** Route-edit calc workbench: reactively loads the parent geofence's area
 *  (`geofence_id`), runs a cluster/bootstrap calc over it, and writes the
 *  ordered result into the route's `geometry`. The golbat category comes from
 *  the route's `mode` (not a panel dropdown). No props — reads/writes the
 *  surrounding route form via react-hook-form context. */
export function RouteMap() {
  const form = useFormContext();
  const geofenceId = useWatch({ name: "geofence_id" }) as number | string | undefined;
  const geometry = useWatch({ name: "geometry" }) as GeoJSON.Geometry | null | undefined;
  const routeMode = useWatch({ name: "mode" }) as string | undefined;
  const category = routeModeToCategory(routeMode);

  // Reactive area: the parent fence, re-fetched when geofence_id changes.
  const { data: fence } = useGetOne("geofence", { id: geofenceId! }, { enabled: geofenceId != null });
  const fenceFeature = useMemo<GeoJSON.Feature | null>(
    () => (fence?.geometry ? { type: "Feature", geometry: fence.geometry as GeoJSON.Geometry, properties: {} } : null),
    [fence],
  );

  // Golbat markers to preview for the route's mode, scoped to the fence polygon.
  // Nothing shows until a geofence is selected (→ the fence loads): `want` gates
  // every fetch on fenceFeature.
  const markerArea = fenceFeature?.geometry ?? null;
  const markerBbox = useMemo<Bounds>(
    () => (fenceFeature?.geometry ? (geometryBounds(fenceFeature.geometry) ?? WORLD) : WORLD),
    [fenceFeature],
  );
  const showCats = routeModeMarkerCategories(routeMode);
  const want = (c: MarkerCategory) => !!fenceFeature && showCats.includes(c);
  const gyms = useMarkers("gym", markerArea, markerBbox, 0, want("gym"));
  const stops = useMarkers("pokestop", markerArea, markerBbox, 0, want("pokestop"));
  const spawns = useMarkers("spawnpoint", markerArea, markerBbox, 0, want("spawnpoint"));
  const stations = useMarkers("station", markerArea, markerBbox, 0, want("station"));

  const calc = useCalc();

  // A succeeded calc result → the route's geometry (MultiPoint of ordered points).
  useEffect(() => {
    if (!calc.result) return;
    const pts = routeCoords(calc.result); // [lon,lat][]
    if (pts.length === 0) return;
    form.setValue("geometry", { type: "MultiPoint", coordinates: pts }, { shouldDirty: true });
  }, [calc.result, form]);

  const onRun = () => {
    if (!fenceFeature) return;
    void calc.run({ area: featureToAreaFC(fenceFeature), category });
  };

  const layers = useMemo<Layer[]>(() => {
    const cats = routeModeMarkerCategories(routeMode);
    const on = (c: MarkerCategory) => !!fenceFeature && cats.includes(c);
    const fenceFC: GeoJSON.FeatureCollection = fenceFeature ? { type: "FeatureCollection", features: [fenceFeature] } : { type: "FeatureCollection", features: [] };
    const routeFC: GeoJSON.FeatureCollection = geometry ? { type: "FeatureCollection", features: [{ type: "Feature", geometry, properties: {} }] } : { type: "FeatureCollection", features: [] };
    const base = buildBaseLayers({
      visibility: { gyms: on("gym"), pokestops: on("pokestop"), spawnpoints: on("spawnpoint"), stations: on("station"), geofences: true, routes: true, s2: false },
      markerSets: [
        { id: "gyms", points: gyms.data ?? [], color: [230, 80, 80], radius: 70, maxPixels: 12 },
        { id: "pokestops", points: stops.data ?? [], color: [0, 120, 255], radius: 40, maxPixels: 6 },
        { id: "spawnpoints", points: spawns.data ?? [], color: [40, 200, 120], radius: 12, maxPixels: 2 },
        { id: "stations", points: stations.data ?? [], color: [150, 80, 220], radius: 40, maxPixels: 6 },
      ],
      geofences: fenceFC, routes: routeFC, s2Cells: [], markerRadius: 70, onClick: () => {}, pickable: false,
      calcResult: calc.result, calcResultIsRoute: true,
      // Cluster circles at the exact calc radius (radius strategy only) — updates
      // live as the user drags the radius.
      calcResultRadius: calc.params.strategy === "radius" ? calc.params.radius : undefined,
    });
    // Loaded route (no active calc result): draw its distance-colored connecting
    // lines. A calc result renders its own path, so skip then to avoid doubling.
    if (!calc.result) {
      const coords =
        geometry?.type === "MultiPoint" || geometry?.type === "LineString"
          ? (geometry.coordinates as [number, number][])
          : [];
      const path = routePathLayer("route-path", coords);
      if (path) return [...base, path];
    }
    return base;
  }, [fenceFeature, geometry, calc.result, gyms.data, stops.data, spawns.data, stations.data, routeMode, calc.params.radius, calc.params.strategy]);

  const fit = geometry ? geometryBounds(geometry) : fenceFeature?.geometry ? geometryBounds(fenceFeature.geometry) : null;
  const areaMissing = !fenceFeature;

  // DeckMap fits its camera once on mount. On CREATE the form starts empty, so
  // remount (via key) when the framing target first appears — a picked fence,
  // then the calc result — so the map isn't stuck at [0,0]. On EDIT geometry is
  // present from the first render, so the key is stable ("geo") → no remount.
  const fitKey = geometry ? "geo" : fenceFeature ? `fence-${geofenceId}` : "empty";

  return (
    <div className="relative">
      <DeckMap key={fitKey} layers={layers} fitBounds={fit} height={640} controller={{ doubleClickZoom: true }}>
        <CalcControls calc={calc} category={category} onRun={onRun} disabled={areaMissing}
          disabledReason={areaMissing ? "Select a geofence first." : undefined} />
      </DeckMap>
    </div>
  );
}
