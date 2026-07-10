import { useEffect, useMemo } from "react";
import { useFormContext, useWatch } from "react-hook-form";
import { useGetOne } from "shadmin-core";
import type { Layer } from "@deck.gl/core";
import { buildBaseLayers } from "@/map/lib/layers";
import { featureToAreaFC, routeCoordsToClusters, AREA_MODES, ROUTE_INPUT_MODES } from "@/map/lib/calc-request";
import { routeCoords } from "@/map/lib/calc-overlay";
import { DeckMap } from "./deck-map";
import { geometryBounds } from "./bounds";
import { useCalc } from "./use-calc";
import { CalcControls } from "./calc-controls";
import { categoryToRouteMode } from "./route-mode";

/** Route-edit calc workbench: reactively loads the parent geofence's area
 *  (`geofence_id`), runs calc (AREA modes over the fence, ROUTE-input modes
 *  over the route's own points), and writes a succeeded result into the
 *  route's `geometry` field. No props — reads/writes the surrounding
 *  route form via react-hook-form context. */
export function RouteMap() {
  const form = useFormContext();
  const geofenceId = useWatch({ name: "geofence_id" }) as number | string | undefined;
  const geometry = useWatch({ name: "geometry" }) as GeoJSON.Geometry | null | undefined;

  // Reactive area: the parent fence, re-fetched when geofence_id changes.
  const { data: fence } = useGetOne("geofence", { id: geofenceId! }, { enabled: geofenceId != null });
  const fenceFeature = useMemo<GeoJSON.Feature | null>(
    () => (fence?.geometry ? { type: "Feature", geometry: fence.geometry as GeoJSON.Geometry, properties: {} } : null),
    [fence],
  );

  const calc = useCalc();

  // A succeeded calc result → the route's geometry (MultiPoint of ordered points).
  useEffect(() => {
    if (!calc.result) return;
    const pts = routeCoords(calc.result); // [lon,lat][]
    if (pts.length === 0) return;
    form.setValue("geometry", { type: "MultiPoint", coordinates: pts }, { shouldDirty: true });
    // category auto-sets the route mode for area modes
    if (AREA_MODES.includes(calc.params.mode)) form.setValue("mode", categoryToRouteMode(calc.params.category), { shouldDirty: true });
  }, [calc.result, calc.params.mode, calc.params.category, form]);

  const onRun = () => {
    if (ROUTE_INPUT_MODES.includes(calc.params.mode)) {
      const feat = geometry ? { type: "Feature" as const, geometry, properties: {} } : null;
      void calc.run({ clusters: routeCoordsToClusters(feat) });
    } else {
      if (!fenceFeature) return;
      void calc.run({ area: featureToAreaFC(fenceFeature) });
    }
  };

  const layers = useMemo<Layer[]>(() => {
    const fenceFC: GeoJSON.FeatureCollection = fenceFeature ? { type: "FeatureCollection", features: [fenceFeature] } : { type: "FeatureCollection", features: [] };
    const routeFC: GeoJSON.FeatureCollection = geometry ? { type: "FeatureCollection", features: [{ type: "Feature", geometry, properties: {} }] } : { type: "FeatureCollection", features: [] };
    return buildBaseLayers({
      visibility: { gyms: false, pokestops: false, spawnpoints: false, stations: false, geofences: true, routes: true, s2: false },
      markerSets: [],
      geofences: fenceFC, routes: routeFC, s2Cells: [], markerRadius: 70, onClick: () => {}, pickable: false,
      calcResult: calc.result, calcResultIsRoute: true,
    });
  }, [fenceFeature, geometry, calc.result]);

  const fit = geometry ? geometryBounds(geometry) : fenceFeature?.geometry ? geometryBounds(fenceFeature.geometry) : null;
  const areaMissing = AREA_MODES.includes(calc.params.mode) && !fenceFeature;
  const routeMissing = ROUTE_INPUT_MODES.includes(calc.params.mode) && !geometry;

  return (
    <div className="relative">
      <DeckMap layers={layers} fitBounds={fit} height={520} controller={{ doubleClickZoom: true }}>
        <div className="absolute top-2 left-2 z-10">
          <CalcControls calc={calc} onRun={onRun} disabled={areaMissing || routeMissing}
            disabledReason={areaMissing ? "Select a geofence first." : routeMissing ? "Route has no points yet." : undefined} />
        </div>
      </DeckMap>
    </div>
  );
}
