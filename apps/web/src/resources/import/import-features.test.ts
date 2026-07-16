import { describe, expect, it } from "vitest";
import { importToFeatures, importFromFeatures } from "./import-features";

const rows = [
  { type: "Feature", geometry: { type: "Polygon", coordinates: [[[0, 0], [1, 0], [1, 1], [0, 0]]] }, properties: { src: "a" }, name: "A", projects: [1] },
  { type: "Feature", geometry: { type: "Polygon", coordinates: [[[2, 2], [3, 2], [3, 3], [2, 2]]] }, properties: {}, name: "B", mode: "quest" },
];

const rows3 = [
  { type: "Feature", geometry: { type: "Polygon", coordinates: [[[0, 0], [1, 0], [1, 1], [0, 0]]] }, properties: {}, name: "A" },
  { type: "Feature", geometry: { type: "Polygon", coordinates: [[[2, 2], [3, 2], [3, 3], [2, 2]]] }, properties: {}, name: "B" },
  { type: "Feature", geometry: { type: "Polygon", coordinates: [[[4, 4], [5, 4], [5, 5], [4, 4]]] }, properties: {}, name: "C" },
];

describe("import step-2 geometry editing round-trip", () => {
  it("stamps _row going out and strips it coming back", () => {
    const feats = importToFeatures(rows);
    expect(feats).toHaveLength(2);
    expect(feats[0].properties).toEqual({ src: "a", _row: rows[0] });
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

  it("a newly drawn feature (no _row) becomes a fresh row", () => {
    const feats = importToFeatures(rows);
    const drawn: GeoJSON.Feature = { type: "Feature", geometry: { type: "Polygon", coordinates: [[[5, 5], [6, 5], [6, 6], [5, 5]]] }, properties: {} };
    const back = importFromFeatures([...feats, drawn], rows) as Record<string, unknown>[];
    expect(back).toHaveLength(3);
    expect(back[2].name).toBeUndefined();
    expect(back[2].geometry).toEqual(drawn.geometry);
  });

  it("edit AFTER a delete keeps the surviving row's assignments (stale-index regression)", () => {
    const feats = importToFeatures(rows3);
    // Commit 1: delete B. prev is still the full 3-row array — this commit was
    // fine even under the old index-stamp design.
    const afterDelete = importFromFeatures([feats[0], feats[2]], rows3) as Record<string, unknown>[];
    expect(afterDelete.map((r) => r.name)).toEqual(["A", "C"]);
    // Commit 2: edit C's geometry. prev is now the SHRUNKEN array — the old
    // _rowIdx design resolved rows[2] === undefined here and dropped C's fields.
    const movedC = { ...feats[2], geometry: { type: "Polygon", coordinates: [[[9, 9], [10, 9], [10, 10], [9, 9]]] } } as GeoJSON.Feature;
    const afterEdit = importFromFeatures([feats[0], movedC], afterDelete) as Record<string, unknown>[];
    expect(afterEdit[1].name).toBe("C");
    expect(afterEdit[1].geometry).toEqual(movedC.geometry);
  });

  it("strips a pre-existing _row key in incoming source properties (we own the key)", () => {
    const dirty = [
      { type: "Feature", geometry: { type: "Point", coordinates: [0, 0] }, properties: { _row: "bogus", keep: 1 }, name: "X" },
    ];
    const feats = importToFeatures(dirty);
    // The bogus incoming value must be replaced by our stamp, not collide with it.
    expect(feats[0].properties?._row).toEqual(dirty[0]);
    expect(feats[0].properties?.keep).toBe(1);
    const back = importFromFeatures(feats, dirty) as Record<string, unknown>[];
    expect(back[0].name).toBe("X");
    expect(back[0].properties).toEqual({ keep: 1 });
  });
});
