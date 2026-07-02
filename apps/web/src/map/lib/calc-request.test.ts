import { expect, test } from "vitest";
import {
  featureToAreaFC,
  routeCoordsToClusters,
  buildCalcBody,
  parseCalcResult,
} from "@/map/lib/calc-request";

test("routeCoordsToClusters transposes geojson [lon,lat] → koji [lat,lon]", () => {
  const line: GeoJSON.Feature = {
    type: "Feature",
    properties: {},
    geometry: { type: "LineString", coordinates: [[-122.3, 47.6], [-122.1, 47.4]] },
  };
  expect(routeCoordsToClusters(line)).toEqual([[47.6, -122.3], [47.4, -122.1]]);
  expect(routeCoordsToClusters(null)).toEqual([]);
});

test("buildCalcBody: cluster carries area + clustering, and mode only when chosen", () => {
  const area = featureToAreaFC({ type: "Feature", properties: {}, geometry: { type: "Point", coordinates: [0, 0] } });
  const bare = buildCalcBody({ mode: "cluster", category: "gym", radius: 70, minPoints: 3 }, { area });
  expect(bare).toEqual({ mode: "cluster", category: "gym", area, clustering: { radius: 70, minPoints: 3 } });

  const withMode = buildCalcBody(
    { mode: "cluster", category: "pokestop", radius: 70, minPoints: 1, clusterMode: "fastest" },
    { area },
  );
  expect((withMode.clustering as Record<string, unknown>).mode).toBe("fastest");
});

test("buildCalcBody: bootstrap uses the bootstrap group; route omits routing (server default)", () => {
  const area = featureToAreaFC({ type: "Feature", properties: {}, geometry: { type: "Point", coordinates: [0, 0] } });
  expect(buildCalcBody({ mode: "bootstrap", category: "pokestop", radius: 90, minPoints: 1 }, { area })).toEqual({
    mode: "bootstrap",
    category: "pokestop",
    area,
    bootstrap: { radius: 90 },
  });
  const route = buildCalcBody({ mode: "route", category: "pokestop", radius: 70, minPoints: 1 }, { area });
  expect(route).not.toHaveProperty("routing");
});

test("buildCalcBody: reroute/routeStats carry clusters ([lat,lon]) not area", () => {
  const clusters: [number, number][] = [[47.6, -122.3], [47.4, -122.1]];
  expect(buildCalcBody({ mode: "reroute", category: "pokestop", radius: 70, minPoints: 1 }, { clusters })).toEqual({
    mode: "reroute",
    clusters,
    radius: 70,
  });
  const stats = buildCalcBody({ mode: "routeStats", category: "pokestop", radius: 70, minPoints: 2 }, { clusters });
  expect(stats).toEqual({ mode: "routeStats", clusters, radius: 70, minPoints: 2 });
});

test("parseCalcResult pulls the FC + stats from a succeeded record", () => {
  const fc: GeoJSON.FeatureCollection = { type: "FeatureCollection", features: [] };
  const r = parseCalcResult({ result: { data: fc, stats: { total_clusters: 5 } } });
  expect(r.fc).toBe(fc);
  expect(r.stats).toEqual({ total_clusters: 5 });
  // Non-FC / missing → null fc, no throw.
  expect(parseCalcResult({ result: null }).fc).toBeNull();
  expect(parseCalcResult(null).fc).toBeNull();
});
