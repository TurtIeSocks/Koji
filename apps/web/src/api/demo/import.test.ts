import { describe, expect, it } from "vitest";
import { postImport } from "./import";
import { allRows } from "./db";
import { resetDemoWorld } from "./seeds/seed";

const SQUARE: GeoJSON.Polygon = {
  type: "Polygon",
  coordinates: [
    [
      [-74.02, 40.7],
      [-74.0, 40.7],
      [-74.0, 40.72],
      [-74.02, 40.72],
      [-74.02, 40.7],
    ],
  ],
};

describe("demo postImport", () => {
  it("dry run reports create vs skip; commit writes", async () => {
    await resetDemoWorld();
    const items = [
      // "Battery Park City" is a seeded neighbourhood → skip.
      {
        kind: "geofence" as const,
        name: "Battery Park City",
        geometry: SQUARE,
        projects: [],
        on_collision: "skip" as const,
      },
      {
        kind: "geofence" as const,
        name: "Brand New",
        geometry: SQUARE,
        projects: [1],
        on_collision: "skip" as const,
      },
    ];
    const dry = await postImport({ dry_run: true, items });
    expect(dry.committed).toBe(false);
    expect(dry.summary).toMatchObject({ create: 1, skip: 1 });
    // Dry run writes nothing.
    const before = await allRows("geofences");
    expect(before.some((r: { name: string }) => r.name === "Brand New")).toBe(false);

    const real = await postImport({ dry_run: false, items });
    expect(real.committed).toBe(true);
    expect(real.summary).toMatchObject({ create: 1, skip: 1 });
    const rows = await allRows("geofences");
    expect(rows.some((r: { name: string }) => r.name === "Brand New")).toBe(true);
    // The created geofence carries the requested project membership.
    const created = rows.find((r: { name: string }) => r.name === "Brand New");
    expect(created?.projects).toEqual([1]);
  });

  it("overwrite collision updates the existing row in place", async () => {
    await resetDemoWorld();
    const before = await allRows("geofences");
    const target = before.find((r) => r.name === "Central Park")!;
    const items = [
      {
        kind: "geofence" as const,
        name: "Central Park",
        geometry: SQUARE,
        projects: [2],
        on_collision: "overwrite" as const,
      },
    ];
    const dry = await postImport({ dry_run: true, items });
    expect(dry.summary).toMatchObject({ update: 1 });

    const real = await postImport({ dry_run: false, items });
    expect(real.summary).toMatchObject({ update: 1 });
    const after = await allRows("geofences");
    // No new row — same id, updated membership.
    expect(after).toHaveLength(before.length);
    const updated = after.find((r) => r.id === target.id)!;
    expect(updated.projects).toEqual([2]);
  });
});
