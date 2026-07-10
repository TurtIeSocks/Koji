import { useEffect, useMemo } from "react";
import { useFormContext, useWatch } from "react-hook-form";
import { useGetOne } from "shadmin-core";
import type { Layer } from "@deck.gl/core";
import { buildBaseLayers } from "@/map/lib/layers";
import { featureToAreaFC } from "@/map/lib/calc-request";
import { routeCoords } from "@/map/lib/calc-overlay";
import { DeckMap } from "./deck-map";
import { geometryBounds } from "./bounds";
import { useCalc } from "./use-calc";
import { CalcControls } from "./calc-controls";
import { routeModeToCategory } from "./route-mode";

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
  const areaMissing = !fenceFeature;

  // DeckMap fits its camera once on mount. On CREATE the form starts empty, so
  // remount (via key) when the framing target first appears — a picked fence,
  // then the calc result — so the map isn't stuck at [0,0]. On EDIT geometry is
  // present from the first render, so the key is stable ("geo") → no remount.
  const fitKey = geometry ? "geo" : fenceFeature ? `fence-${geofenceId}` : "empty";

  return (
    <div className="relative">
      <DeckMap key={fitKey} layers={layers} fitBounds={fit} height={520} controller={{ doubleClickZoom: true }}>
        <div className="absolute top-2 left-2 z-10">
          <CalcControls calc={calc} category={category} onRun={onRun} disabled={areaMissing}
            disabledReason={areaMissing ? "Select a geofence first." : undefined} />
        </div>
      </DeckMap>
    </div>
  );
}
