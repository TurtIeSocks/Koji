import { useMemo } from "react";
import { FormProvider, useForm, useFormContext, useWatch } from "react-hook-form";
import { Link } from "react-router";
import { ArrowLeft } from "lucide-react";
import type { Layer } from "@deck.gl/core";
import { buildBaseLayers } from "@/map/lib/layers";
import { featureToAreaFC } from "@/map/lib/calc-request";
import { useMarkers } from "@/map/data/use-markers";
import type { Bounds, MarkerCategory } from "@/map/stores/types";
import { useStartCenter } from "@/lib/use-start-center";
import { Button } from "@/components/ui/button";
import { DeckGeoJsonInput } from "./deck-geojson-input";
import { geometryBounds } from "./bounds";
import { geofenceToFeatures, featuresToGeofence } from "./geofence-geometry";
import { useCalc } from "./use-calc";
import { CalcControls } from "./calc-controls";
import { routeModeToCategory, routeModeMarkerCategories } from "./route-mode";

const WORLD: Bounds = [-180, -85, 180, 85];
const EMPTY_FC: GeoJSON.FeatureCollection = { type: "FeatureCollection", features: [] };
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
  const geometry = useWatch({ name: "geometry" }) as GeoJSON.Geometry | null | undefined;
  const mode = useWatch({ name: "mode" }) as string;
  const category = routeModeToCategory(mode);

  const [startLat, startLon] = useStartCenter();

  // Markers scoped to the drawn area, by data mode. Nothing until something's drawn.
  const area = geometry ?? null;
  const bbox = useMemo<Bounds>(() => (geometry ? geometryBounds(geometry) ?? WORLD : WORLD), [geometry]);
  const showCats = routeModeMarkerCategories(mode);
  const want = (c: MarkerCategory) => !!geometry && showCats.includes(c);
  const gyms = useMarkers("gym", area, bbox, 0, want("gym"));
  const stops = useMarkers("pokestop", area, bbox, 0, want("pokestop"));
  const spawns = useMarkers("spawnpoint", area, bbox, 0, want("spawnpoint"));
  const stations = useMarkers("station", area, bbox, 0, want("station"));

  const calc = useCalc();

  const onRun = () => {
    if (!geometry) return;
    void calc.run({ area: featureToAreaFC({ type: "Feature", geometry, properties: {} }), category });
  };

  const contextLayers = useMemo<Layer[]>(() => {
    const cats = routeModeMarkerCategories(mode);
    const on = (c: MarkerCategory) => !!geometry && cats.includes(c);
    return buildBaseLayers({
      visibility: { gyms: on("gym"), pokestops: on("pokestop"), spawnpoints: on("spawnpoint"), stations: on("station"), geofences: false, routes: false, s2: false },
      markerSets: [
        { id: "gyms", points: gyms.data ?? [], color: [230, 80, 80], radius: 70, maxPixels: 12 },
        { id: "pokestops", points: stops.data ?? [], color: [0, 120, 255], radius: 40, maxPixels: 6 },
        { id: "spawnpoints", points: spawns.data ?? [], color: [40, 200, 120], radius: 12, maxPixels: 2 },
        { id: "stations", points: stations.data ?? [], color: [150, 80, 220], radius: 40, maxPixels: 6 },
      ],
      geofences: EMPTY_FC, routes: EMPTY_FC, s2Cells: [], markerRadius: 70, onClick: () => {}, pickable: false,
      calcResult: calc.result, calcResultIsRoute: true,
      calcResultRadius: calc.params.strategy === "radius" ? calc.params.radius : undefined,
    });
  }, [geometry, mode, gyms.data, stops.data, spawns.data, stations.data, calc.result, calc.params.radius, calc.params.strategy]);

  const overlay = (
    <>
      <CalcControls
        calc={calc}
        category={category}
        onRun={onRun}
        disabled={!geometry}
        disabledReason={!geometry ? "Draw an area first" : undefined}
      />
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
        <Button asChild size="sm" variant="secondary" className="bg-background/95 shadow-sm backdrop-blur">
          <Link to="/">
            <ArrowLeft className="size-4" /> Admin
          </Link>
        </Button>
      </div>
    </>
  );

  return (
    <DeckGeoJsonInput
      source="geometry"
      height="100dvh"
      contextLayers={contextLayers}
      toFeatures={geofenceToFeatures}
      fromFeatures={featuresToGeofence}
      defaultViewState={{ longitude: startLon, latitude: startLat, zoom: 11 }}
      overlay={overlay}
    />
  );
}
