import { describe, expect, it } from "vitest";
import { GEOFENCE_MODES, GEOMETRY_TYPES } from "@/lib/constants";

describe("geofence constants", () => {
  it("collapses to the 4 canonical v2 modes", () => {
    expect(GEOFENCE_MODES.map((m) => m.id).sort()).toEqual([
      "fort",
      "pokemon",
      "quest",
      "unset",
    ]);
  });

  it("offers only Polygon and MultiPolygon geometry types", () => {
    expect(GEOMETRY_TYPES.map((g) => g.id)).toEqual(["Polygon", "MultiPolygon"]);
  });
});
