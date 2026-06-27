import { beforeEach, expect, test } from "vitest";
import { useMapSettingsStore } from "@/map/stores/map-settings-store";

beforeEach(() => {
  localStorage.clear();
  useMapSettingsStore.setState({ tileServerId: "default", markerRadius: 30 });
});

test("defaults are present", () => {
  const s = useMapSettingsStore.getState();
  expect(s.tileServerId).toBe("default");
  expect(s.defaultLayerVisibility.geofences).toBe(true);
  expect(s.areaThresholds.pokestop).toBeGreaterThan(0);
});

test("setTileServerId persists to localStorage under koji-map-settings", () => {
  useMapSettingsStore.getState().setTileServerId("osm-bright");
  expect(useMapSettingsStore.getState().tileServerId).toBe("osm-bright");
  const raw = localStorage.getItem("koji-map-settings");
  expect(raw).toContain("osm-bright");
});
