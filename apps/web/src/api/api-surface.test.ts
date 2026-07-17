import { describe, expect, it } from "vitest";
import type { ApiSurface } from "./types";
import * as live from "./index.live";

describe("@api surface", () => {
  it("live implements ApiSurface", () => {
    const surface: ApiSurface = live; // compile-time check
    expect(surface.baseDataProvider.getList).toBeTypeOf("function");
    expect(surface.submitCalc).toBeTypeOf("function");
    expect(surface.resetDemo).toBeTypeOf("function");
  });
});
