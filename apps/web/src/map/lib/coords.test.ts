import { expect, test } from "vitest";
import { boundsToBboxArg, fromKojiLatLon, packMarkers } from "@/map/lib/coords";
import type { Bounds } from "@/map/stores/types";

test("fromKojiLatLon transposes [lat,lon] -> [lng,lat]", () => {
  expect(fromKojiLatLon([47.5, -122.3])).toEqual([-122.3, 47.5]);
});

test("packMarkers produces a stride-2 [lng,lat,...] Float32Array", () => {
  const out = packMarkers([[47.5, -122.3], [10, 20]]);
  expect(out).toBeInstanceOf(Float32Array);
  expect(out.length).toBe(4);
  expect(Array.from(out)).toEqual([-122.3, 47.5, 20, 10].map((n) => Math.fround(n)));
});

test("boundsToBboxArg maps [minLng,minLat,maxLng,maxLat] to Koji lat/lon snake_case", () => {
  const b: Bounds = [-122.4, 47.4, -122.2, 47.6];
  expect(boundsToBboxArg(b)).toEqual({
    min_lat: 47.4, min_lon: -122.4, max_lat: 47.6, max_lon: -122.2,
  });
});
