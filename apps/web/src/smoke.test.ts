import { describe, expect, it } from "vitest";

describe("scaffold", () => {
  it("resolves the @ alias and runs vitest", async () => {
    const mod = await import("@/App");
    expect(typeof mod.default).toBe("function");
  });
});
