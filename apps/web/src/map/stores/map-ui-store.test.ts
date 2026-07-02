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

test("editFeature loads a geofence into the modify editor, id from top-level feature.id", () => {
  // The REAL server shape: koji serializes the geofence id as the geojson
  // top-level `feature.id` (koji_geojson.rs), NOT properties.id.
  const feature: GeoJSON.Feature = {
    type: "Feature",
    id: 42,
    properties: { name: "test_tight", mode: "Unset" },
    geometry: { type: "Polygon", coordinates: [[[0, 0], [1, 0], [1, 1], [0, 0]]] },
  };
  useMapUIStore.getState().editFeature(feature);
  const s = useMapUIStore.getState();
  expect(s.drawMode).toBe("modify");
  expect(s.draftFeatures.features).toEqual([feature]);
  expect(s.selectedFeatureIndexes).toEqual([0]);
  expect(s.editingGeofenceId).toBe("42");
  // clearDraft resets the editing id so a later Save creates, not updates
  useMapUIStore.getState().clearDraft();
  expect(useMapUIStore.getState().editingGeofenceId).toBeNull();
});

test("editFeature falls back to properties.id when there's no top-level id", () => {
  const feature: GeoJSON.Feature = {
    type: "Feature",
    properties: { id: 7 },
    geometry: { type: "Polygon", coordinates: [[[0, 0], [1, 0], [1, 1], [0, 0]]] },
  };
  useMapUIStore.getState().editFeature(feature);
  expect(useMapUIStore.getState().editingGeofenceId).toBe("7");
});
