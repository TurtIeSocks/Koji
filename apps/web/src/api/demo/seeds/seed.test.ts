// jsdom (the unit-test environment) has no real IndexedDB. Rather than
// depend on that omission — a jsdom implementation detail that could
// change — force the in-memory fallback explicitly by stubbing
// `indexedDB` out before any demo-db call runs. db.ts only decides which
// backend to use lazily, on first actual read/write, so this stub (set at
// module load, before any `it` body executes) is guaranteed to win.
import { describe, expect, it, vi } from "vitest";
import { allRows } from "../db";
import { ensureSeeded, resetDemoWorld, seedRoutes } from "./seed";

vi.stubGlobal("indexedDB", undefined);

describe("seed", () => {
  it("seeds 32 geofences + 3 projects and is idempotent", async () => {
    await ensureSeeded();
    await ensureSeeded(); // second call must not duplicate
    const fences = await allRows("geofences");
    expect(fences).toHaveLength(32);
    expect(await allRows("projects")).toHaveLength(3);
    const modes = new Set(fences.map((f: { mode: string }) => f.mode));
    expect(modes).toEqual(new Set(["pokemon", "fort", "quest"]));
    for (const f of fences) expect(f.projects.length).toBeGreaterThan(0);
  });
  it("resetDemoWorld reseeds", async () => {
    await resetDemoWorld();
    expect(await allRows("geofences")).toHaveLength(32);
  });
});

describe("seedRoutes", () => {
  // A fake facade: submitCalc returns a fixed id, getJob returns a succeeded
  // record whose result FC carries two points → the seeded route's MultiPoint.
  const fakeCalc = () => {
    const fc: GeoJSON.FeatureCollection = {
      type: "FeatureCollection",
      features: [
        { type: "Feature", properties: {}, geometry: { type: "Point", coordinates: [-74, 40.71] } },
        { type: "Feature", properties: {}, geometry: { type: "Point", coordinates: [-73.99, 40.72] } },
      ],
    };
    return {
      submitCalc: vi.fn(async () => "demo-1"),
      getJob: vi.fn(async () => ({ status: "succeeded", result: { data: fc, stats: {} } })),
    };
  };

  it("stores a MultiPoint route per seed fence with the expected shape", async () => {
    await resetDemoWorld();
    const { submitCalc, getJob } = fakeCalc();
    await seedRoutes(submitCalc, getJob);
    const routes = await allRows("routes");
    expect(routes).toHaveLength(2); // fences 1 & 2
    expect(submitCalc).toHaveBeenCalledTimes(2);
    const first = routes.find((r) => r.geofence_id === 1)!;
    expect(first.name).toMatch(/ Route$/);
    expect(first.mode).toBe("quest");
    expect(first.geometry.type).toBe("MultiPoint");
    expect(first.points).toBe(2);
  });

  it("is idempotent — a second call runs no calc when routes already exist", async () => {
    await resetDemoWorld();
    const first = fakeCalc();
    await seedRoutes(first.submitCalc, first.getJob);
    const second = fakeCalc();
    await seedRoutes(second.submitCalc, second.getJob);
    expect(second.submitCalc).not.toHaveBeenCalled();
    expect(await allRows("routes")).toHaveLength(2);
  });

  it("skips a fence (no throw) when its calc fails", async () => {
    await resetDemoWorld();
    const submitCalc = vi.fn(async () => "demo-1");
    const getJob = vi.fn(async () => ({ status: "failed", result: null, error: "boom" }));
    await expect(seedRoutes(submitCalc, getJob)).resolves.toBeUndefined();
    expect(await allRows("routes")).toHaveLength(0);
  });
});
