import { useMemo } from "react";
import { Map as MapLibre, useControl } from "react-map-gl/maplibre";
import { MapboxOverlay } from "@deck.gl/mapbox";
import "maplibre-gl/dist/maplibre-gl.css";
import { DEFAULT_TILE_URL } from "@/lib/constants";
import { rasterStyle } from "@/map/lib/map-style";
import { buildLayers } from "@/map/lib/layers";
import { useMapViewStore } from "@/map/stores/map-view-store";
import { useMapUIStore } from "@/map/stores/map-ui-store";
import { useMapSettingsStore } from "@/map/stores/map-settings-store";
import { useMarkers } from "@/map/data/use-markers";
import { useGeoFeatures } from "@/map/data/use-geo-features";
import { useS2Cells } from "@/map/data/use-s2-cells";
import type { Bounds } from "@/map/stores/types";

function DeckOverlay({ layers }: { layers: ReturnType<typeof buildLayers> }) {
  const overlay = useControl(() => new MapboxOverlay({ interleaved: false, layers }));
  (overlay as MapboxOverlay).setProps?.({ layers });
  return null;
}

export function DeckCanvas() {
  // Render-subscriptions: narrow primitives only (S3).
  const visibility = useMapUIStore((s) => s.layerVisibility);
  const s2Level = useMapUIStore((s) => s.s2Level);
  const setSelection = useMapUIStore((s) => s.setSelection);
  const markerRadius = useMapSettingsStore((s) => s.markerRadius);
  const tileServerId = useMapSettingsStore((s) => s.tileServerId); // (Phase 1: maps to DEFAULT_TILE_URL)

  // settledBounds drives the fetches; subscribing to it re-renders ~5×/sec, not per frame.
  const bounds = useMapViewStore((s) => s.settledBounds);

  const gyms = useMarkers("gym", bounds, 0, visibility.gyms);
  const stops = useMarkers("pokestop", bounds, 0, visibility.pokestops);
  const spawns = useMarkers("spawnpoint", bounds, 0, visibility.spawnpoints);
  const stations = useMarkers("station", bounds, 0, visibility.stations);
  const geofences = useGeoFeatures("geofences", visibility.geofences);
  const routes = useGeoFeatures("routes", visibility.routes);
  const s2 = useS2Cells(s2Level, bounds, visibility.s2);

  const layers = useMemo(
    () =>
      buildLayers({
        visibility,
        markerSets: [
          { id: "gyms", points: gyms.data ?? [], color: [230, 80, 80] },
          { id: "pokestops", points: stops.data ?? [], color: [0, 120, 255] },
          { id: "spawnpoints", points: spawns.data ?? [], color: [240, 180, 0] },
          { id: "stations", points: stations.data ?? [], color: [150, 80, 220] },
        ],
        geofences: geofences.data ?? { type: "FeatureCollection", features: [] },
        routes: routes.data ?? { type: "FeatureCollection", features: [] },
        s2CellIds: s2.data ?? [],
        markerRadius,
        onClick: (info) => {
          const id = info.layer?.id ?? "";
          if (id.startsWith("markers-")) setSelection({ kind: "marker", id: String(info.index) });
          else if (id === "geofences") setSelection({ kind: "geofence", id: String(info.index) });
          else if (id === "routes") setSelection({ kind: "route", id: String(info.index) });
        },
      }),
    [visibility, gyms.data, stops.data, spawns.data, stations.data, geofences.data, routes.data, s2.data, markerRadius, setSelection],
  );

  const mapStyle = useMemo(() => rasterStyle(DEFAULT_TILE_URL), [tileServerId]);

  return (
    <MapLibre
      initialViewState={{ longitude: 0, latitude: 0, zoom: 2 }}
      mapStyle={mapStyle}
      onMove={(e: { viewState: { longitude: number; latitude: number; zoom: number; pitch: number; bearing: number } }) => {
        const vs = e.viewState;
        // Transient hot-path write: no component subscribes to liveViewState via a hook.
        const b = boundsFromViewState(vs);
        useMapViewStore.getState().setLive(vs, b);
      }}
      style={{ width: "100%", height: "100%" }}
    >
      <DeckOverlay layers={layers} />
    </MapLibre>
  );
}

/** Approximate bounds from a viewState when the map instance isn't queried directly.
 *  Phase 1 uses the map's own getBounds via onMove event target where available;
 *  this fallback keeps the fetch keyed on a stable bbox. */
function boundsFromViewState(vs: { longitude: number; latitude: number; zoom: number }): Bounds {
  const span = 360 / 2 ** vs.zoom;
  return [vs.longitude - span, vs.latitude - span / 2, vs.longitude + span, vs.latitude + span / 2];
}
