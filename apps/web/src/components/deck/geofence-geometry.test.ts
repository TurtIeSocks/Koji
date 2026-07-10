import { describe, expect, it } from "vitest";
import { geofenceToFeatures, featuresToGeofence } from "./geofence-geometry";

const P1 = [[[0, 0], [1, 0], [1, 1], [0, 1], [0, 0]]];
const P2 = [[[2, 2], [3, 2], [3, 3], [2, 3], [2, 2]]];

describe("geofenceToFeatures", () => {
  it("expands a MultiPolygon into one Polygon feature per part", () => {
    const features = geofenceToFeatures({ type: "MultiPolygon", coordinates: [P1, P2] });
    expect(features).toHaveLength(2);
    expect(features[0].geometry).toEqual({ type: "Polygon", coordinates: P1 });
    expect(features[1].geometry).toEqual({ type: "Polygon", coordinates: P2 });
  });

  it("wraps a lone Polygon in a single feature", () => {
    const features = geofenceToFeatures({ type: "Polygon", coordinates: P1 });
    expect(features).toHaveLength(1);
    expect(features[0].geometry).toEqual({ type: "Polygon", coordinates: P1 });
  });

  it("returns no features for null/empty", () => {
    expect(geofenceToFeatures(null)).toEqual([]);
    expect(geofenceToFeatures(undefined)).toEqual([]);
  });
});

describe("featuresToGeofence", () => {
  it("combines Polygon features into a MultiPolygon (so a 2nd drawn fence is kept)", () => {
    const geom = featuresToGeofence([
      { type: "Feature", geometry: { type: "Polygon", coordinates: P1 }, properties: {} },
      { type: "Feature", geometry: { type: "Polygon", coordinates: P2 }, properties: {} },
    ]);
    expect(geom).toEqual({ type: "MultiPolygon", coordinates: [P1, P2] });
  });

  it("persists a lone Polygon as a 1-part MultiPolygon", () => {
    const geom = featuresToGeofence([
      { type: "Feature", geometry: { type: "Polygon", coordinates: P1 }, properties: {} },
    ]);
    expect(geom).toEqual({ type: "MultiPolygon", coordinates: [P1] });
  });

  it("round-trips a MultiPolygon losslessly", () => {
    const original = { type: "MultiPolygon" as const, coordinates: [P1, P2] };
    expect(featuresToGeofence(geofenceToFeatures(original))).toEqual(original);
  });
});
