import { describe, expect, it } from "vitest";
import type { ApiSurface } from "./types";
import * as live from "./index.live";
// Imported directly (not via the `@api` alias, which tsconfig/vitest keep
// pointed at index.live.ts) — this is the only place index.demo.ts gets
// type-checked against ApiSurface until a real demo build resolves it.
import * as demo from "./index.demo";

describe("@api surface", () => {
  it("live implements ApiSurface", () => {
    const surface: ApiSurface = live; // compile-time check
    expect(surface.baseDataProvider.getList).toBeTypeOf("function");
    expect(surface.submitCalc).toBeTypeOf("function");
    expect(surface.resetDemo).toBeTypeOf("function");
  });

  it("demo implements ApiSurface", () => {
    const surface: ApiSurface = demo; // compile-time check
    expect(surface.baseDataProvider.getList).toBeTypeOf("function");
    expect(surface.submitCalc).toBeTypeOf("function");
    expect(surface.resetDemo).toBeTypeOf("function");
  });
});
