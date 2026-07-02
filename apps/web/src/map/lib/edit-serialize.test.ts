import { expect, test } from "vitest";
import { firstGeometry, allGeometries, shouldCommitEdit } from "@/map/lib/edit-serialize";

test("firstGeometry returns the geometry of the first drawn feature, or null", () => {
  const geom: GeoJSON.Polygon = { type: "Polygon", coordinates: [[[0, 0], [1, 0], [1, 1], [0, 0]]] };
  const fc: GeoJSON.FeatureCollection = {
    type: "FeatureCollection",
    features: [{ type: "Feature", properties: {}, geometry: geom }],
  };
  expect(firstGeometry(fc)).toEqual(geom);
  expect(firstGeometry({ type: "FeatureCollection", features: [] })).toBeNull();
});

test("allGeometries returns every drawn feature's geometry", () => {
  const a: GeoJSON.Polygon = { type: "Polygon", coordinates: [[[0, 0], [1, 0], [1, 1], [0, 0]]] };
  const b: GeoJSON.Polygon = { type: "Polygon", coordinates: [[[2, 2], [3, 2], [3, 3], [2, 2]]] };
  const fc: GeoJSON.FeatureCollection = {
    type: "FeatureCollection",
    features: [
      { type: "Feature", properties: {}, geometry: a },
      { type: "Feature", properties: {}, geometry: b },
    ],
  };
  expect(allGeometries(fc)).toEqual([a, b]);
  expect(allGeometries({ type: "FeatureCollection", features: [] })).toEqual([]);
});

// Guards the draw-lag fix: tentative cursor-follow edits must NOT commit to the
// store (they fire hundreds of times/polygon). Regression here re-introduces jank.
test("shouldCommitEdit skips tentative cursor-follow edits", () => {
  expect(shouldCommitEdit("updateTentativeFeature")).toBe(false);
  expect(shouldCommitEdit("addTentativePosition")).toBe(false);
});

test("shouldCommitEdit commits real geometry edits", () => {
  for (const t of ["addFeature", "addPosition", "removePosition", "finishMovePosition", "translating", "rotating", "scaling"]) {
    expect(shouldCommitEdit(t)).toBe(true);
  }
  // Unknown/undefined editType commits (matches the original guard's default).
  expect(shouldCommitEdit(undefined)).toBe(true);
});
