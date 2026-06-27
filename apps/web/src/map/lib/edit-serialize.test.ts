import { expect, test } from "vitest";
import { firstGeometry } from "@/map/lib/edit-serialize";

test("firstGeometry returns the geometry of the first drawn feature, or null", () => {
  const geom: GeoJSON.Polygon = { type: "Polygon", coordinates: [[[0, 0], [1, 0], [1, 1], [0, 0]]] };
  const fc: GeoJSON.FeatureCollection = {
    type: "FeatureCollection",
    features: [{ type: "Feature", properties: {}, geometry: geom }],
  };
  expect(firstGeometry(fc)).toEqual(geom);
  expect(firstGeometry({ type: "FeatureCollection", features: [] })).toBeNull();
});
