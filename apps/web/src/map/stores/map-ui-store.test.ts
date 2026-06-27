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

test("setDraftFeatures + clearDraft manage the edit buffer", () => {
  const fc: GeoJSON.FeatureCollection = {
    type: "FeatureCollection",
    features: [{ type: "Feature", properties: {}, geometry: { type: "Point", coordinates: [1, 2] } }],
  };
  useMapUIStore.getState().setDrawMode("drawPolygon");
  useMapUIStore.getState().setDraftFeatures(fc);
  useMapUIStore.getState().setSelectedFeatureIndexes([0]);
  expect(useMapUIStore.getState().draftFeatures.features).toHaveLength(1);
  useMapUIStore.getState().clearDraft();
  expect(useMapUIStore.getState().draftFeatures.features).toHaveLength(0);
  expect(useMapUIStore.getState().selectedFeatureIndexes).toEqual([]);
  expect(useMapUIStore.getState().drawMode).toBe("none");
});

test("filter actions update lastSeen and tth independently", () => {
  useMapUIStore.getState().setLastSeen(3600);
  useMapUIStore.getState().setTth("Known");
  expect(useMapUIStore.getState().filters).toEqual({ lastSeen: 3600, tth: "Known" });
});
