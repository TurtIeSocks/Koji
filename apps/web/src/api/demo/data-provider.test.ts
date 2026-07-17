import { beforeAll, describe, expect, it } from "vitest";
import { demoBaseDataProvider } from "./data-provider";
import { ensureSeeded } from "./seeds/seed";

beforeAll(async () => {
  await ensureSeeded();
});

describe("demo dataProvider", () => {
  it("lists geofences with pagination + sort + total", async () => {
    const r = await demoBaseDataProvider.getList("geofence", {
      pagination: { page: 1, perPage: 10 },
      sort: { field: "name", order: "ASC" },
      filter: {},
    });
    expect(r.total).toBe(32);
    expect(r.data).toHaveLength(10);
    expect(r.data[0].name <= r.data[1].name).toBe(true);
    expect(r.data[0]).toHaveProperty("projects");
    expect(r.data[0]).toHaveProperty("property_count");
  });

  it("filters by mode and q", async () => {
    const r = await demoBaseDataProvider.getList("geofence", {
      pagination: { page: 1, perPage: 50 },
      sort: { field: "id", order: "ASC" },
      filter: { mode: "quest", q: "park" },
    });
    for (const row of r.data) {
      expect(row.mode).toBe("quest");
      expect(row.name.toLowerCase()).toContain("park");
    }
  });

  it("getOne geofence returns a flattened record the admin can render", async () => {
    const r = await demoBaseDataProvider.getOne("geofence", { id: 1 });
    // Demo returns the already-flattened record (the ApiSurface contract is
    // ra-core DataProvider, whose getOne returns records — see task brief).
    expect(r.data.id).toBe(1);
    expect(r.data.geometry?.type).toBe("Polygon");
    expect(r.data.geo_type).toBe("Polygon");
  });

  it("create/update/delete round-trip a route", async () => {
    const created = await demoBaseDataProvider.create("route", {
      data: {
        name: "t-route",
        geofence_id: 1,
        mode: "quest",
        geometry: {
          type: "MultiPoint",
          coordinates: [
            [-74.0, 40.7],
            [-74.01, 40.71],
          ],
        },
      },
    });
    expect(created.data.id).toBeGreaterThan(0);
    // points computed from the geometry coordinate count.
    expect(created.data.points).toBe(2);

    const upd = await demoBaseDataProvider.update("route", {
      id: created.data.id,
      data: { name: "t-route-2" },
      previousData: created.data,
    });
    expect(upd.data.name).toBe("t-route-2");
    // update is a merge — geometry survives a partial patch.
    expect(upd.data.geometry?.type).toBe("MultiPoint");

    await demoBaseDataProvider.delete("route", { id: created.data.id });
    const list = await demoBaseDataProvider.getList("route", {
      pagination: { page: 1, perPage: 50 },
      sort: { field: "id", order: "ASC" },
      filter: { q: "t-route" },
    });
    expect(list.data).toHaveLength(0);
  });

  it("plugins list is always empty", async () => {
    const r = await demoBaseDataProvider.getList("plugins", {
      pagination: { page: 1, perPage: 10 },
      sort: { field: "id", order: "ASC" },
      filter: {},
    });
    expect(r).toEqual({ data: [], total: 0 });
  });

  it("filters routes by geofenceid and points range", async () => {
    const a = await demoBaseDataProvider.create("route", {
      data: {
        name: "geo-1-route",
        geofence_id: 1,
        mode: "quest",
        geometry: { type: "MultiPoint", coordinates: [[-74, 40.7]] },
      },
    });
    const b = await demoBaseDataProvider.create("route", {
      data: {
        name: "geo-2-route",
        geofence_id: 2,
        mode: "quest",
        geometry: {
          type: "MultiPoint",
          coordinates: [
            [-74, 40.7],
            [-74, 40.71],
            [-74, 40.72],
          ],
        },
      },
    });
    const scoped = await demoBaseDataProvider.getList("route", {
      pagination: { page: 1, perPage: 50 },
      sort: { field: "id", order: "ASC" },
      filter: { geofenceid: 2 },
    });
    expect(scoped.data.every((r) => r.geofence_id === 2)).toBe(true);
    expect(scoped.data.some((r) => r.id === b.data.id)).toBe(true);

    const bigger = await demoBaseDataProvider.getList("route", {
      pagination: { page: 1, perPage: 50 },
      sort: { field: "id", order: "ASC" },
      filter: { pointsmin: 2 },
    });
    expect(bigger.data.every((r) => r.points >= 2)).toBe(true);

    await demoBaseDataProvider.delete("route", { id: a.data.id });
    await demoBaseDataProvider.delete("route", { id: b.data.id });
  });

  it("hydrates project.geofences from geofence membership", async () => {
    const r = await demoBaseDataProvider.getOne("project", { id: 1 });
    expect(Array.isArray(r.data.geofences)).toBe(true);
    // Project 1 (Downtown) owns at least one seeded member geofence.
    expect(r.data.geofences.length).toBeGreaterThan(0);
  });
});
