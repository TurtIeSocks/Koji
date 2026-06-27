import { expect, test, vi } from "vitest";
import { buildLayers } from "@/map/lib/layers";
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
    geofences: EMPTY, routes: EMPTY, s2CellIds: [],
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
    geofences: EMPTY, routes: EMPTY, s2CellIds: ["abc"],
    markerRadius: 30, onClick: vi.fn(),
  });
  expect(layers.find((l) => l.id === "s2")!.props.visible).toBe(false);
});
