import { describe, expect, it } from "vitest";
import { mapToPairs, pairsToMap } from "./headers-input";

describe("mapToPairs", () => {
  it("converts a header map to name/value rows", () => {
    expect(mapToPairs({ "x-golbat-secret": "abc", Authorization: "Bearer t" })).toEqual([
      { name: "x-golbat-secret", value: "abc" },
      { name: "Authorization", value: "Bearer t" },
    ]);
  });
  it("returns [] for null/undefined/empty", () => {
    expect(mapToPairs(null)).toEqual([]);
    expect(mapToPairs(undefined)).toEqual([]);
    expect(mapToPairs({})).toEqual([]);
  });
});

describe("pairsToMap", () => {
  it("converts name/value rows to a header map", () => {
    expect(pairsToMap([{ name: "X-A", value: "1" }, { name: "X-B", value: "2" }])).toEqual({
      "X-A": "1",
      "X-B": "2",
    });
  });
  it("drops rows with an empty/whitespace name", () => {
    expect(pairsToMap([{ name: "", value: "x" }, { name: "  ", value: "y" }, { name: "X", value: "z" }])).toEqual({ X: "z" });
  });
  it("last-wins on duplicate keys", () => {
    expect(pairsToMap([{ name: "X", value: "1" }, { name: "X", value: "2" }])).toEqual({ X: "2" });
  });
  it("returns {} for null/undefined", () => {
    expect(pairsToMap(null)).toEqual({});
    expect(pairsToMap(undefined)).toEqual({});
  });
});
