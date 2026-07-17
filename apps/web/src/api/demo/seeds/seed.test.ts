// jsdom (the unit-test environment) has no real IndexedDB. Rather than
// depend on that omission — a jsdom implementation detail that could
// change — force the in-memory fallback explicitly by stubbing
// `indexedDB` out before any demo-db call runs. db.ts only decides which
// backend to use lazily, on first actual read/write, so this stub (set at
// module load, before any `it` body executes) is guaranteed to win.
import { describe, expect, it, vi } from "vitest";
import { allRows } from "../db";
import { ensureSeeded, resetDemoWorld } from "./seed";

vi.stubGlobal("indexedDB", undefined);

describe("seed", () => {
  it("seeds 32 geofences + 3 projects and is idempotent", async () => {
    await ensureSeeded();
    await ensureSeeded(); // second call must not duplicate
    const fences = await allRows("geofences");
    expect(fences).toHaveLength(32);
    expect(await allRows("projects")).toHaveLength(3);
    const modes = new Set(fences.map((f: { mode: string }) => f.mode));
    expect(modes).toEqual(new Set(["pokemon", "fort", "quest"]));
    for (const f of fences) expect(f.projects.length).toBeGreaterThan(0);
  });
  it("resetDemoWorld reseeds", async () => {
    await resetDemoWorld();
    expect(await allRows("geofences")).toHaveLength(32);
  });
});
