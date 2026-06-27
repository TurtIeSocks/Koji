import { useMemo, useState, useEffect } from "react";
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
import { useMapRealtime } from "@/map/data/use-map-realtime";
import type { Bounds } from "@/map/stores/types";

function DeckOverlay({ layers }: { layers: ReturnType<typeof buildLayers> }) {
  const overlay = useControl(() => new MapboxOverlay({ interleaved: false, layers }));
  (overlay as MapboxOverlay).setProps?.({ layers });
  return null;
}

export function DeckCanvas() {
  // Subscribe to geofence/route realtime deltas → refetch GeoJSON layers.
  useMapRealtime();

  // DeckCanvas is the layer AGGREGATOR: it rebuilds the whole layer list and so
  // legitimately needs every visibility flag. Subscribing to the whole
  // `layerVisibility` map is correct here (not the prop-drill anti-pattern) — it
  // re-renders only when a flag actually toggles (selection/hover writes don't
  // touch this slice, so they don't churn it). Per-field S3 selectors live in
  // the leaf panels (<LayerToggle>), where render isolation actually matters.
  const visibility = useMapUIStore((s) => s.layerVisibility);
  const s2Level = useMapUIStore((s) => s.s2Level);
  const setSelection = useMapUIStore((s) => s.setSelection);
  const drawMode = useMapUIStore((s) => s.drawMode);
  const draftFeatures = useMapUIStore((s) => s.draftFeatures);
  const selectedFeatureIndexes = useMapUIStore((s) => s.selectedFeatureIndexes);
  const setDraftFeatures = useMapUIStore((s) => s.setDraftFeatures);
  const markerRadius = useMapSettingsStore((s) => s.markerRadius);
  const tileServerId = useMapSettingsStore((s) => s.tileServerId); // (Phase 1: maps to DEFAULT_TILE_URL)

  // Filter state — per-field S3 subscriptions.
  const lastSeenLive = useMapUIStore((s) => s.filters.lastSeen);
  const tth = useMapUIStore((s) => s.filters.tth);

  // Debounce lastSeen 400ms so slider drags don't spam the server.
  const [lastSeen, setLastSeenDebounced] = useState(lastSeenLive);
  useEffect(() => {
    const id = setTimeout(() => setLastSeenDebounced(lastSeenLive), 400);
    return () => clearTimeout(id);
  }, [lastSeenLive]);

  // settledBounds drives the fetches; subscribing to it re-renders ~5×/sec, not per frame.
  const bounds = useMapViewStore((s) => s.settledBounds);

  const gyms = useMarkers("gym", bounds, lastSeen, visibility.gyms);
  const stops = useMarkers("pokestop", bounds, lastSeen, visibility.pokestops);
  const spawns = useMarkers("spawnpoint", bounds, lastSeen, visibility.spawnpoints, tth);
  const stations = useMarkers("station", bounds, lastSeen, visibility.stations);
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
        s2Cells: s2.data ?? [],
        markerRadius,
        onClick: (info) => {
          const id = info.layer?.id ?? "";
          if (id.startsWith("markers-")) setSelection({ kind: "marker", id: String(info.index) });
          else if (id === "geofences") setSelection({ kind: "geofence", id: String(info.index) });
          else if (id === "routes") setSelection({ kind: "route", id: String(info.index) });
        },
        draft: {
          mode: drawMode,
          features: draftFeatures,
          selectedIndexes: selectedFeatureIndexes,
          onEdit: (e) => setDraftFeatures(e.updatedData),
        },
      }),
    [visibility, gyms.data, stops.data, spawns.data, stations.data, geofences.data, routes.data, s2.data, markerRadius, setSelection, drawMode, draftFeatures, selectedFeatureIndexes, setDraftFeatures],
  );

  const mapStyle = useMemo(() => rasterStyle(DEFAULT_TILE_URL), [tileServerId]);

  return (
    <MapLibre
      initialViewState={{ longitude: 0, latitude: 0, zoom: 2 }}
      mapStyle={mapStyle}
      onMove={(e: {
        viewState: { longitude: number; latitude: number; zoom: number; pitch: number; bearing: number };
        target?: { getBounds?: () => { getWest(): number; getSouth(): number; getEast(): number; getNorth(): number } };
      }) => {
        const vs = e.viewState;
        // Prefer the map's real viewport bounds; fall back to an approximation
        // only when the instance isn't queryable (e.g. the headless test mock).
        const lb = e.target?.getBounds?.();
        const b: Bounds = lb
          ? [lb.getWest(), lb.getSouth(), lb.getEast(), lb.getNorth()]
          : boundsFromViewState(vs);
        // Transient hot-path write: no component subscribes to liveViewState via a hook.
        useMapViewStore.getState().setLive(vs, b);
      }}
      style={{ width: "100%", height: "100%" }}
    >
      <DeckOverlay layers={layers} />
    </MapLibre>
  );
}

/** Fallback bounds approximation used only when the live map can't be queried
 *  (the headless test mock). The real path reads `e.target.getBounds()`. */
function boundsFromViewState(vs: { longitude: number; latitude: number; zoom: number }): Bounds {
  const span = 360 / 2 ** vs.zoom;
  return [vs.longitude - span, vs.latitude - span / 2, vs.longitude + span, vs.latitude + span / 2];
}
