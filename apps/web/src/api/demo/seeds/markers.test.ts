import { describe, expect, it } from "vitest";
import nyc from "./nyc-areas.geo.json";
import { generateMarkers } from "./markers";
import { pointInPolygon } from "./geometry";

// `nyc` is typed straight off the JSON literal's own content (tsc's
// "bundler" module resolution parses .json imports even without
// `resolveJsonModule` set — geometry.type et al. come back as plain
// `string`, not the literal GeoJSON.Geometry union needs), so a single
// `as` doesn't sufficiently overlap; go through `unknown`.
const fc = nyc as unknown as GeoJSON.FeatureCollection;

describe("generateMarkers", () => {
  it("is deterministic", () => {
    const a = generateMarkers(fc).query("spawnpoint", {});
    const b = generateMarkers(fc).query("spawnpoint", {});
    expect(a).toEqual(b);
  });
  it("generates all four categories with sane counts", () => {
    const store = generateMarkers(fc);
    const sp = store.query("spawnpoint", {});
    expect(sp.length).toBeGreaterThan(5000);
    expect(store.query("gym", {}).length).toBeGreaterThan(300);
    expect(store.query("pokestop", {}).length).toBeGreaterThan(1000);
    expect(store.query("station", {}).length).toBeGreaterThan(100);
  });
  it("all points fall inside their source polygons' union bbox", () => {
    const store = generateMarkers(fc);
    for (const [lat, lon] of store.query("gym", {})) {
      expect(lat).toBeGreaterThan(40.69);
      expect(lat).toBeLessThan(40.89);
      expect(lon).toBeGreaterThan(-74.03);
      expect(lon).toBeLessThan(-73.9);
    }
  });
  it("bbox query filters", () => {
    const store = generateMarkers(fc);
    const all = store.query("spawnpoint", {});
    const some = store.query("spawnpoint", {
      bbox: { minLat: 40.7, minLon: -74.02, maxLat: 40.72, maxLon: -74.0 },
    });
    expect(some.length).toBeGreaterThan(0);
    expect(some.length).toBeLessThan(all.length);
  });
  it("area query keeps only in-polygon points", () => {
    const store = generateMarkers(fc);
    const feat = fc.features[0];
    const pts = store.query("spawnpoint", { areaFeatures: [feat] });
    const ring = (feat.geometry as GeoJSON.Polygon).coordinates;
    for (const p of pts.slice(0, 50)) expect(pointInPolygon(p, ring)).toBe(true);
  });
  it("tth filter partitions spawnpoints", () => {
    const store = generateMarkers(fc);
    const all = store.query("spawnpoint", {}).length;
    const known = store.query("spawnpoint", { tth: "Known" }).length;
    const unknown = store.query("spawnpoint", { tth: "Unknown" }).length;
    expect(known + unknown).toBe(all);
    expect(known).toBeGreaterThan(0);
    expect(unknown).toBeGreaterThan(0);
  });
});
