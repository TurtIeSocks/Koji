import { describe, expect, it } from "vitest";
import { importToFeatures, importFromFeatures } from "./import-features";

const rows = [
  { type: "Feature", geometry: { type: "Polygon", coordinates: [[[0, 0], [1, 0], [1, 1], [0, 0]]] }, properties: { src: "a" }, name: "A", projects: [1] },
  { type: "Feature", geometry: { type: "Polygon", coordinates: [[[2, 2], [3, 2], [3, 3], [2, 2]]] }, properties: {}, name: "B", mode: "quest" },
];

describe("import step-2 geometry editing round-trip", () => {
  it("stamps _rowIdx going out and strips it coming back", () => {
    const feats = importToFeatures(rows);
    expect(feats).toHaveLength(2);
    expect(feats[0].properties).toEqual({ src: "a", _rowIdx: 0 });
    const back = importFromFeatures(feats, rows) as typeof rows;
    expect(back[0].properties).toEqual({ src: "a" });
  });

  it("a geometry edit lands on the right row, assignments intact", () => {
    const feats = importToFeatures(rows);
    const moved: GeoJSON.Feature = { ...feats[1], geometry: { type: "Polygon", coordinates: [[[9, 9], [10, 9], [10, 10], [9, 9]]] } };
    const back = importFromFeatures([feats[0], moved], rows) as typeof rows;
    expect(back[1].geometry).toEqual(moved.geometry);
    expect(back[1].name).toBe("B");
    expect(back[1].mode).toBe("quest");
  });

  it("deleting feature 0 drops row 0 and keeps row 1's assignments (index shift)", () => {
    const feats = importToFeatures(rows);
    const back = importFromFeatures([feats[1]], rows) as typeof rows;
    expect(back).toHaveLength(1);
    expect(back[0].name).toBe("B");
  });

  it("a newly drawn feature (no _rowIdx) becomes a fresh row", () => {
    const feats = importToFeatures(rows);
    const drawn: GeoJSON.Feature = { type: "Feature", geometry: { type: "Polygon", coordinates: [[[5, 5], [6, 5], [6, 6], [5, 5]]] }, properties: {} };
    const back = importFromFeatures([...feats, drawn], rows) as Record<string, unknown>[];
    expect(back).toHaveLength(3);
    expect(back[2].name).toBeUndefined();
    expect(back[2].geometry).toEqual(drawn.geometry);
  });
});
