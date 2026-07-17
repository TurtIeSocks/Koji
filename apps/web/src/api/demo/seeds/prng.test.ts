import { describe, expect, it } from "vitest";
import { mulberry32 } from "./prng";

describe("mulberry32", () => {
  it("is deterministic per seed", () => {
    const a = mulberry32(42),
      b = mulberry32(42);
    expect([a(), a(), a()]).toEqual([b(), b(), b()]);
  });
  it("stays in [0,1)", () => {
    const r = mulberry32(7);
    for (let i = 0; i < 1000; i++) {
      const v = r();
      expect(v).toBeGreaterThanOrEqual(0);
      expect(v).toBeLessThan(1);
    }
  });
});
