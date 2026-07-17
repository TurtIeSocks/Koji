import { beforeAll, describe, expect, it } from "vitest";
import {
  fetchFeatureCollection,
  fetchGeofencesByBbox,
  fetchGeofencesByIds,
} from "./geo-features";
import { ensureSeeded } from "./seeds/seed";

beforeAll(async () => {
  await ensureSeeded();
});

describe("demo geo-features", () => {
  it("bbox query intersects stored bboxes", async () => {
    // Bounds is [minLon, minLat, maxLon, maxLat] — Lower Manhattan.
    const fc = await fetchGeofencesByBbox([-74.02, 40.7, -74.0, 40.72]);
    expect(fc.type).toBe("FeatureCollection");
    expect(fc.features.length).toBeGreaterThan(0);
    expect(fc.features.length).toBeLessThan(32);
    for (const f of fc.features) expect(f.properties).toHaveProperty("name");
  });

  it("bbox query honours the mode filter", async () => {
    const fc = await fetchGeofencesByBbox([-74.5, 40.6, -73.5, 40.9], {
      mode: "quest",
    });
    expect(fc.features.length).toBeGreaterThan(0);
    for (const f of fc.features) expect(f.properties?.mode).toBe("quest");
  });

  it("fetchFeatureCollection returns one feature per geofence row", async () => {
    const fc = await fetchFeatureCollection("geofences");
    expect(fc.type).toBe("FeatureCollection");
    expect(fc.features).toHaveLength(32);
    for (const f of fc.features) {
      expect(f.properties).toHaveProperty("id");
      expect(f.properties).toHaveProperty("name");
      expect(f.properties).toHaveProperty("mode");
    }
  });

  it("fetchGeofencesByIds scopes to the requested ids", async () => {
    const fc = await fetchGeofencesByIds([1, 2, 3]);
    expect(fc.features).toHaveLength(3);
    const ids = fc.features.map((f) => f.properties?.id).sort();
    expect(ids).toEqual([1, 2, 3]);
  });
});
