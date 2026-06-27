import { beforeEach, expect, test } from "vitest";
import { useMapUIStore } from "@/map/stores/map-ui-store";

beforeEach(() => useMapUIStore.setState(useMapUIStore.getInitialState()));

test("toggleLayer flips exactly one layer and leaves others untouched", () => {
  const before = useMapUIStore.getState().layerVisibility.gyms;
  useMapUIStore.getState().toggleLayer("gyms");
  expect(useMapUIStore.getState().layerVisibility.gyms).toBe(!before);
  expect(useMapUIStore.getState().layerVisibility.geofences).toBe(true);
});

test("setSelection stores kind + id", () => {
  useMapUIStore.getState().setSelection({ kind: "geofence", id: "42" });
  expect(useMapUIStore.getState().selection).toEqual({ kind: "geofence", id: "42" });
});
