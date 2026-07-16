import { describe, expect, it } from "vitest";
import { geometryBounds, boundsToViewState, valueBounds } from "./bounds";

describe("geometryBounds", () => {
  it("returns [minLng,minLat,maxLng,maxLat] for a polygon", () => {
    const poly: GeoJSON.Polygon = {
      type: "Polygon",
      coordinates: [[[10, 20], [12, 20], [12, 22], [10, 22], [10, 20]]],
    };
    expect(geometryBounds(poly)).toEqual([10, 20, 12, 22]);
  });
  it("returns null for empty/invalid geometry", () => {
    expect(geometryBounds({ type: "GeometryCollection", geometries: [] })).toBeNull();
  });
});

describe("boundsToViewState", () => {
  it("centers on the bounds midpoint", () => {
    const vs = boundsToViewState([10, 20, 12, 22], 800, 600);
    expect(vs.longitude).toBeCloseTo(11, 1);
    expect(vs.latitude).toBeCloseTo(21, 1);
    expect(vs.zoom).toBeGreaterThan(0);
  });
});

describe("valueBounds", () => {
  const polyA: GeoJSON.Feature<GeoJSON.Polygon> = {
    type: "Feature",
    properties: {},
    geometry: {
      type: "Polygon",
      coordinates: [[[10, 20], [12, 20], [12, 22], [10, 22], [10, 20]]],
    },
  };
  const polyB: GeoJSON.Feature<GeoJSON.Polygon> = {
    type: "Feature",
    properties: {},
    geometry: {
      type: "Polygon",
      coordinates: [[[30, 40], [32, 40], [32, 42], [30, 42], [30, 40]]],
    },
  };

  // Regressive: the import wizard binds `source="features"` to a BARE ARRAY
  // of feature-shaped row objects (not a FeatureCollection). turf's bbox()
  // throws "Unknown Geometry Type" on a raw array; `geometryBounds` catches
  // and returns null, which locked the wizard's step-2 map at null island.
  // This case FAILS if `valueBounds` is reverted to a plain `geometryBounds`
  // call on the array.
  it("computes bounds enclosing all features for a bare array of Features", () => {
    expect(valueBounds([polyA, polyB])).toEqual([10, 20, 32, 42]);
  });

  it("matches the array case when wrapped in a FeatureCollection", () => {
    const fc: GeoJSON.FeatureCollection = {
      type: "FeatureCollection",
      features: [polyA, polyB],
    };
    expect(valueBounds(fc)).toEqual(valueBounds([polyA, polyB]));
  });

  it("computes bounds for a bare Geometry", () => {
    expect(valueBounds(polyA.geometry)).toEqual([10, 20, 12, 22]);
  });

  it("returns null for null", () => {
    expect(valueBounds(null)).toBeNull();
  });

  it("returns null for an empty array", () => {
    expect(valueBounds([])).toBeNull();
  });
});
