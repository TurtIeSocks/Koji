import { expect, test, vi } from "vitest";
import { buildEditLayer, buildLayers, routePathLayer } from "@/map/lib/layers";
import { KojiDrawPolygonMode } from "@/map/lib/koji-draw-polygon-mode";
import type { LayerId } from "@/map/stores/types";

const vis = (over: Partial<Record<LayerId, boolean>> = {}): Record<LayerId, boolean> => ({
  gyms: false, pokestops: true, spawnpoints: false, stations: false,
  geofences: true, routes: false, s2: false, ...over,
});
const EMPTY: GeoJSON.FeatureCollection = { type: "FeatureCollection", features: [] };

test("buildLayers emits one visible scatter layer per visible marker set + wires onClick", () => {
  const onClick = vi.fn();
  const layers = buildLayers({
    visibility: vis(),
    markerSets: [{ id: "pokestops", points: [[47.5, -122.3]], color: [0, 120, 255] }],
    geofences: EMPTY, routes: EMPTY, s2Cells: [],
    markerRadius: 30, onClick,
  });
  const stop = layers.find((l) => l.id === "markers-pokestops");
  expect(stop).toBeDefined();
  expect(stop!.props.visible).toBe(true);
  // the click handler is wired through to every pickable layer
  expect(stop!.props.onClick).toBe(onClick);
  expect(layers.find((l) => l.id === "geofences")!.props.onClick).toBe(onClick);
  expect(layers.find((l) => l.id === "routes")!.props.onClick).toBe(onClick);
  // geofences visible, routes hidden
  expect(layers.find((l) => l.id === "geofences")!.props.visible).toBe(true);
  expect(layers.find((l) => l.id === "routes")!.props.visible).toBe(false);
});

test("S2 layer is present but hidden when s2 visibility is off", () => {
  const layers = buildLayers({
    visibility: vis({ s2: false }), markerSets: [],
    geofences: EMPTY, routes: EMPTY,
    s2Cells: [{ id: "abc", ring: [[-122.3, 47.5], [-122.2, 47.6]] }],
    markerRadius: 30, onClick: vi.fn(),
  });
  expect(layers.find((l) => l.id === "s2")!.props.visible).toBe(false);
});

test("the geofence being edited is dimmed; other geofences stay orange", () => {
  const layers = buildLayers({
    visibility: vis(), markerSets: [], geofences: EMPTY, routes: EMPTY, s2Cells: [],
    markerRadius: 30, onClick: vi.fn(), editingGeofenceId: "42",
  });
  const geo = layers.find((l) => l.id === "geofences")!;
  const getFill = (geo.props as unknown as { getFillColor: (f: GeoJSON.Feature) => number[] }).getFillColor;
  const pt: GeoJSON.Point = { type: "Point", coordinates: [0, 0] };
  const editing: GeoJSON.Feature = { type: "Feature", id: 42, properties: {}, geometry: pt };
  const other: GeoJSON.Feature = { type: "Feature", id: 7, properties: {}, geometry: pt };
  expect(getFill(editing)).toEqual([130, 130, 130, 25]); // dimmed
  expect(getFill(other)).toEqual([255, 140, 0, 40]); // orange
});

test("calc cluster circles use the exact meter radius, and only when calcResultRadius is set", () => {
  const fc: GeoJSON.FeatureCollection = {
    type: "FeatureCollection",
    features: [{ type: "Feature", geometry: { type: "MultiPoint", coordinates: [[-122.3, 47.5]] }, properties: {} }],
  };
  const withRadius = buildLayers({
    visibility: vis(), markerSets: [], geofences: EMPTY, routes: EMPTY, s2Cells: [],
    markerRadius: 30, onClick: vi.fn(),
    calcResult: fc, calcResultIsRoute: true, calcResultRadius: 70,
  });
  const circles = withRadius.find((l) => l.id === "calc-circles");
  expect(circles).toBeDefined();
  expect((circles!.props as unknown as { radiusUnits: string }).radiusUnits).toBe("meters");
  expect((circles!.props as unknown as { getRadius: number }).getRadius).toBe(70);

  // No radius (e.g. S2 strategy) → no coverage circles.
  const noRadius = buildLayers({
    visibility: vis(), markerSets: [], geofences: EMPTY, routes: EMPTY, s2Cells: [],
    markerRadius: 30, onClick: vi.fn(), calcResult: fc, calcResultIsRoute: true,
  });
  expect(noRadius.find((l) => l.id === "calc-circles")).toBeUndefined();
});

test("routePathLayer builds a segment layer, and is null under 2 points", () => {
  expect(routePathLayer("route-path", [[0, 0]])).toBeNull();
  const layer = routePathLayer("route-path", [[0, 0], [1, 1], [2, 0]]);
  expect(layer).not.toBeNull();
  expect(layer!.id).toBe("route-path");
});

test("buildLayers appends an editable layer only when a draw mode is active", () => {
  const draftOff = buildLayers({
    visibility: vis(), markerSets: [], geofences: EMPTY, routes: EMPTY, s2Cells: [],
    markerRadius: 30, onClick: vi.fn(),
    draft: { mode: "none", features: EMPTY, selectedIndexes: [], onEdit: vi.fn() },
  });
  expect(draftOff.find((l) => l.id.startsWith("edit"))).toBeUndefined();

  const draftOn = buildLayers({
    visibility: vis(), markerSets: [], geofences: EMPTY, routes: EMPTY, s2Cells: [],
    markerRadius: 30, onClick: vi.fn(),
    draft: { mode: "drawPolygon", features: EMPTY, selectedIndexes: [], onEdit: vi.fn() },
  });
  expect(draftOn.find((l) => l.id.startsWith("edit"))).toBeDefined();
});

test("buildEditLayer refreshes a KojiDrawPolygonMode instance's stash on every rebuild", () => {
  // The mode instance's own click-driven stash is otherwise only refreshed by
  // canvas pointer events — a toolbar action (undo/redo/delete) rebuilds the
  // draft without one. buildEditLayer must call syncDraft so finish()/cancel()
  // never emit against a stale pre-undo FeatureCollection (Task 10 review finding).
  const modeInstance = new KojiDrawPolygonMode();
  const syncDraft = vi.spyOn(modeInstance, "syncDraft");
  const onEdit = vi.fn();
  const features: GeoJSON.FeatureCollection = { type: "FeatureCollection", features: [] };
  buildEditLayer({ mode: "drawPolygon", features, selectedIndexes: [], onEdit, modeInstance });
  expect(syncDraft).toHaveBeenCalledWith(features, onEdit);

  // A non-KojiDrawPolygonMode instance (e.g. plain modify) has no syncDraft —
  // buildEditLayer must not blow up reaching for it.
  expect(() =>
    buildEditLayer({ mode: "modify", features, selectedIndexes: [], onEdit, modeInstance: {} }),
  ).not.toThrow();
});
