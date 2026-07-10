import { describe, expect, it } from "vitest";
import { geometryBounds, boundsToViewState } from "./bounds";

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
