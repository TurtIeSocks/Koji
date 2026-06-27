import { expect, test } from "vitest";
import { mergeSelected } from "@/map/lib/merge-polygons";

const poly = (x: number): GeoJSON.Feature => ({
  type: "Feature", properties: {},
  geometry: { type: "Polygon", coordinates: [[[x, 0], [x + 2, 0], [x + 2, 2], [x, 2], [x, 0]]] },
});

test("mergeSelected unions two overlapping polygons into one feature", () => {
  const fc: GeoJSON.FeatureCollection = { type: "FeatureCollection", features: [poly(0), poly(1)] };
  const out = mergeSelected(fc, [0, 1]);
  expect(out.features).toHaveLength(1);
  expect(["Polygon", "MultiPolygon"]).toContain(out.features[0].geometry.type);
});

test("mergeSelected with <2 indexes returns the input unchanged", () => {
  const fc: GeoJSON.FeatureCollection = { type: "FeatureCollection", features: [poly(0)] };
  expect(mergeSelected(fc, [0])).toBe(fc);
});
