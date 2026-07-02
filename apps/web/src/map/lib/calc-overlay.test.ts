import { expect, test } from "vitest";
import { routeCoords, routeSegments, rampColor, segmentColors } from "@/map/lib/calc-overlay";

test("routeCoords extracts ordered coords from a MultiPoint result", () => {
  const fc: GeoJSON.FeatureCollection = {
    type: "FeatureCollection",
    features: [{ type: "Feature", properties: {}, geometry: { type: "MultiPoint", coordinates: [[0, 0], [1, 1], [2, 0]] } }],
  };
  expect(routeCoords(fc)).toEqual([[0, 0], [1, 1], [2, 0]]);
  expect(routeCoords(null)).toEqual([]);
  // a non-route (single Point) → no path
  expect(routeCoords({ type: "FeatureCollection", features: [{ type: "Feature", properties: {}, geometry: { type: "Point", coordinates: [0, 0] } }] })).toEqual([]);
});

test("routeSegments yields n-1 consecutive segments", () => {
  const segs = routeSegments([[0, 0], [1, 0], [3, 0]]);
  expect(segs).toHaveLength(2);
  expect(segs[0].source).toEqual([0, 0]);
  expect(segs[0].target).toEqual([1, 0]);
  // the second leg (0→3 span 2) is longer than the first (0→1 span 1)
  expect(segs[1].length).toBeGreaterThan(segs[0].length);
});

test("rampColor goes green → yellow → red", () => {
  expect(rampColor(0)).toEqual([0, 255, 0]);
  expect(rampColor(0.5)).toEqual([255, 255, 0]);
  expect(rampColor(1)).toEqual([255, 0, 0]);
});

test("segmentColors reddens the longest leg, greens the shortest", () => {
  const segs = routeSegments([[0, 0], [1, 0], [4, 0]]); // legs of length ~1 and ~3
  const colors = segmentColors(segs);
  expect(colors[0]).toEqual([0, 255, 0]); // shortest → green
  expect(colors[1]).toEqual([255, 0, 0]); // longest → red
});
