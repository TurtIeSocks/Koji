import { describe, expect, it } from "vitest";
import { featuresToImportItems } from "./to-import-items";

describe("featuresToImportItems", () => {
  it("maps a Polygon feature to a geofence item", () => {
    const result = featuresToImportItems([
      {
        geometry: { type: "Polygon" },
        name: "A",
        mode: "pokemon",
        on_collision: "skip",
        projects: [1],
      },
    ]);
    expect(result).toHaveLength(1);
    const item = result[0];
    expect(item.kind).toBe("geofence");
    expect(item.name).toBe("A");
    expect(item.geometry).toEqual({ type: "Polygon" });
    expect(item.mode).toBe("pokemon");
    expect(item.projects).toEqual([1]);
    expect(item.on_collision).toBe("skip");
    expect("route_parent" in item).toBe(false);
    expect("parent" in item).toBe(false);
  });

  it("maps a MultiPoint feature to a route item with route_parent", () => {
    const result = featuresToImportItems([
      {
        geometry: { type: "MultiPoint" },
        name: "R",
        route_parent: "7",
      },
    ]);
    const item = result[0];
    expect(item.kind).toBe("route");
    expect(item.route_parent).toBe("7");
    expect(item.projects).toEqual([]);
    expect(item.on_collision).toBe("skip");
  });

  it("kind override wins over geometry type", () => {
    const result = featuresToImportItems([
      {
        geometry: { type: "Polygon" },
        kind: "route",
        name: "X",
      },
    ]);
    expect(result[0].kind).toBe("route");
  });

  it("fills defaults for a minimal feature", () => {
    const result = featuresToImportItems([
      { geometry: { type: "Polygon" }, name: "Y" },
    ]);
    const item = result[0];
    expect(item.projects).toEqual([]);
    expect(item.on_collision).toBe("skip");
    expect("mode" in item).toBe(false);
  });
});
