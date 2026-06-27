import { useMemo, useState, useEffect, useCallback } from "react";
import DeckGL from "@deck.gl/react";
import { Map as MapLibre } from "react-map-gl/maplibre";
import { WebMercatorViewport } from "@deck.gl/core";
import type { PickingInfo } from "@deck.gl/core";
import "maplibre-gl/dist/maplibre-gl.css";
import { DEFAULT_TILE_URL } from "@/lib/constants";
import { rasterStyle } from "@/map/lib/map-style";
import { buildBaseLayers, buildEditLayer } from "@/map/lib/layers";
import { useMapViewStore } from "@/map/stores/map-view-store";
import { useMapUIStore } from "@/map/stores/map-ui-store";
import { useMapSettingsStore } from "@/map/stores/map-settings-store";
import { useMarkers } from "@/map/data/use-markers";
import { useGeoFeatures } from "@/map/data/use-geo-features";
import { useS2Cells } from "@/map/data/use-s2-cells";
import { useMapRealtime } from "@/map/data/use-map-realtime";
import type { Bounds } from "@/map/stores/types";

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
  const setSelectedFeatureIndexes = useMapUIStore((s) => s.setSelectedFeatureIndexes);
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

  // Stable so the base-layer memo isn't invalidated every render.
  const handleClick = useCallback(
    (info: PickingInfo) => {
      const id = info.layer?.id ?? "";
      if (id.startsWith("markers-")) setSelection({ kind: "marker", id: String(info.index) });
      else if (id === "geofences") setSelection({ kind: "geofence", id: String(info.index) });
      else if (id === "routes") setSelection({ kind: "route", id: String(info.index) });
    },
    [setSelection],
  );

  // Base data layers — rebuilt ONLY when their data/visibility change, NOT on
  // every edit click (that would repack the marker Float32Array each vertex).
  const baseLayers = useMemo(
    () =>
      buildBaseLayers({
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
        onClick: handleClick,
      }),
    [visibility, gyms.data, stops.data, spawns.data, stations.data, geofences.data, routes.data, s2.data, markerRadius, handleClick],
  );

  // Edit layer — the only thing that rebuilds on a per-click draft change.
  const editLayer = useMemo(
    () =>
      buildEditLayer({
        mode: drawMode,
        features: draftFeatures,
        selectedIndexes: selectedFeatureIndexes,
        onEdit: (e) => setDraftFeatures(e.updatedData),
        onSelect: setSelectedFeatureIndexes,
      }),
    [drawMode, draftFeatures, selectedFeatureIndexes, setDraftFeatures, setSelectedFeatureIndexes],
  );

  const layers = useMemo(() => [...baseLayers, ...editLayer], [baseLayers, editLayer]);

  const mapStyle = useMemo(() => rasterStyle(DEFAULT_TILE_URL), [tileServerId]);

  // deck.gl is the INTERACTION ROOT (controller), MapLibre is a child that syncs
  // to deck's viewState. This is required for @deck.gl-community/editable-layers:
  // as a MapboxOverlay, deck only gets a subset of inputs (no onDrag) so drawing
  // breaks. doubleClickZoom is off so a double-click FINISHES a drawn polygon.
  return (
    <DeckGL
      initialViewState={{ longitude: 0, latitude: 0, zoom: 2 }}
      controller={{ doubleClickZoom: false }}
      layers={layers}
      getCursor={({ isDragging }) =>
        drawMode !== "none" ? "crosshair" : isDragging ? "grabbing" : "grab"
      }
      onViewStateChange={(params) => {
        const vs = params.viewState as unknown as ViewStateLike;
        // Transient hot-path write: no component subscribes to liveViewState via a hook.
        useMapViewStore.getState().setLive(toViewState(vs), boundsOf(vs));
      }}
    >
      <MapLibre mapStyle={mapStyle} reuseMaps />
    </DeckGL>
  );
}

interface ViewStateLike {
  longitude: number;
  latitude: number;
  zoom: number;
  pitch?: number;
  bearing?: number;
}

function toViewState(vs: ViewStateLike) {
  return { longitude: vs.longitude, latitude: vs.latitude, zoom: vs.zoom, pitch: vs.pitch ?? 0, bearing: vs.bearing ?? 0 };
}

/** Real viewport bounds from the deck view; falls back to an approximation if the
 *  WebMercator projection can't be built (e.g. the headless test with no window). */
function boundsOf(vs: ViewStateLike): Bounds {
  const width = typeof window !== "undefined" ? window.innerWidth || 800 : 800;
  const height = typeof window !== "undefined" ? window.innerHeight || 600 : 600;
  try {
    const [w, s, e, n] = new WebMercatorViewport({ ...toViewState(vs), width, height }).getBounds();
    return [w, s, e, n];
  } catch {
    const span = 360 / 2 ** vs.zoom;
    return [vs.longitude - span, vs.latitude - span / 2, vs.longitude + span, vs.latitude + span / 2];
  }
}
