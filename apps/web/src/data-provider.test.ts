import { describe, expect, it } from "vitest";
import { baseDataProvider, serializeGeofenceWrite } from "@/data-provider";

describe("serializeGeofenceWrite", () => {
  it("strips read-only keys from each properties row to {property_id, value}", () => {
    const out = serializeGeofenceWrite({
      name: "F",
      mode: "unset",
      properties: [
        { id: 100, geofence_id: 9, property_id: 1, name: "is_event", category: "boolean", value: true },
        { property_id: 2, value: { a: 1 } },
      ],
    });
    expect(out.properties).toEqual([
      { property_id: 1, value: true },
      { property_id: 2, value: { a: 1 } },
    ]);
    // Non-properties fields pass through untouched.
    expect(out.name).toBe("F");
    expect(out.mode).toBe("unset");
  });

  it("passes data through unchanged when properties is absent", () => {
    const data = { name: "F", projects: [1, 2] };
    expect(serializeGeofenceWrite(data)).toEqual(data);
  });
});

describe("baseDataProvider geofence", () => {
  it("getList returns flat rows + total from meta (no featureToRecord)", async () => {
    const res = await baseDataProvider.getList("geofence", {
      pagination: { page: 1, perPage: 10 },
      sort: { field: "name", order: "ASC" },
      filter: {},
    });
    expect(res.total).toBe(2);
    expect(res.data[0]).toMatchObject({ id: 1, name: "Alpha", mode: "pokemon", geo_type: "Polygon" });
  });

  it("getList forwards the q filter", async () => {
    const res = await baseDataProvider.getList("geofence", {
      pagination: { page: 1, perPage: 10 },
      sort: { field: "name", order: "ASC" },
      filter: { q: "beta" },
    });
    expect(res.data).toHaveLength(1);
    expect(res.data[0].name).toBe("Beta");
  });

  it("getOne maps a single Feature to a record (lossless)", async () => {
    const res = await baseDataProvider.getOne("geofence", { id: 1 });
    expect(res.data).toMatchObject({ id: 1, name: "Alpha", mode: "pokemon", geo_type: "Polygon" });
    expect(res.data.geometry.type).toBe("Polygon");
  });

  it("getList for a macro resource unwraps {data,total}", async () => {
    const res = await baseDataProvider.getList("project", {
      pagination: { page: 1, perPage: 10 },
      sort: { field: "id", order: "ASC" },
      filter: {},
    });
    expect(res.total).toBe(1);
    expect(res.data[0]).toMatchObject({ id: 10, name: "Proj-A" });
  });

  it("create posts to /internal/geofences and returns the new id", async () => {
    const res = await baseDataProvider.create("geofence", {
      data: { name: "Gamma", mode: "unset" },
    });
    expect(res.data.id).toBe(3);
  });

  it("delete returns the deleted id on 204", async () => {
    const res = await baseDataProvider.delete("geofence", { id: 2, previousData: { id: 2 } });
    expect(res.data.id).toBe(2);
  });
});
