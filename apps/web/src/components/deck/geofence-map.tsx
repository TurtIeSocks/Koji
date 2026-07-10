import { useMemo, useState } from "react";
import { useWatch } from "react-hook-form";
import type { Layer } from "@deck.gl/core";
import { buildBaseLayers } from "@/map/lib/layers";
import { useMarkers } from "@/map/data/use-markers";
import { useS2Cells } from "@/map/data/use-s2-cells";
import type { Bounds } from "@/map/stores/types";
import { Button } from "@/components/ui/button";
import { DeckGeoJsonInput } from "./deck-geojson-input";
import { geometryBounds } from "./bounds";
import { geofenceToFeatures, featuresToGeofence } from "./geofence-geometry";

const WORLD: Bounds = [-180, -85, 180, 85];

export function GeofenceMap() {
  // Live geometry from the form drives the marker query as the fence is edited.
  // Markers use the actual polygon `area` (so a MultiPolygon returns points
  // inside its parts, not the whole bbox); S2 cells stay bbox-based (a grid).
  const geometry = useWatch({ name: "geometry" }) as GeoJSON.Geometry | null | undefined;
  const area = geometry ?? null;
  const bbox = useMemo<Bounds>(() => (geometry ? (geometryBounds(geometry) ?? WORLD) : WORLD), [geometry]);

  const [show, setShow] = useState({ gyms: false, pokestops: false, spawnpoints: false, s2: false });
  const gyms = useMarkers("gym", area, bbox, 0, show.gyms);
  const stops = useMarkers("pokestop", area, bbox, 0, show.pokestops);
  const spawns = useMarkers("spawnpoint", area, bbox, 0, show.spawnpoints);
  const s2 = useS2Cells(15, bbox, show.s2);

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
        // Per-category radius (meters) — spawnpoints are dense, so much smaller;
        // pokestops a bit smaller; gyms keep the default. Tune here.
        markerSets: [
          { id: "gyms", points: gyms.data ?? [], color: [230, 80, 80], radius: 70 },
          { id: "pokestops", points: stops.data ?? [], color: [0, 120, 255], radius: 40 },
          { id: "spawnpoints", points: spawns.data ?? [], color: [240, 180, 0], radius: 12 },
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
      <div className="flex flex-wrap gap-1">
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
      </div>
      <DeckGeoJsonInput
        source="geometry"
        label="Geometry"
        height={480}
        contextLayers={contextLayers}
        toFeatures={geofenceToFeatures}
        fromFeatures={featuresToGeofence}
      />
    </div>
  );
}
