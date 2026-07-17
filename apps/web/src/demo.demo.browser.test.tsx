// Demo-mode end-to-end smoke test, in a real (Playwright) browser, against
// the REAL wasm-pack build — no fakes, no mocked worker. This is the one spot
// in the suite that exercises the actual `crates/koji-wasm` binary the demo
// site ships, catching anything a jsdom unit test (fake `postCalc`, see
// `src/api/demo/calc/facade.test.ts`) or a mocked-worker browser test can't:
// a stale/broken wasm build, a worker that fails to boot, or a wire-shape
// mismatch between the JS facade and the actual Rust `calc()` export.
//
// Requires `crates/koji-wasm/pkg` (gitignored, built via `bun run wasm:build`)
// — see vitest.config.ts's `demo-browser` project, which computes
// `__WASM_PKG_PRESENT__` from that file's presence on disk at config-eval
// time (a real browser test file can't stat the filesystem itself).
//
// `@api` (index.demo.ts) transitively, statically imports `src/api/demo/wasm.ts`,
// which contains a (lazy-called, but statically-parsed) `import("@koji-wasm")`.
// Vite resolves/rewrites every import specifier — static AND dynamic-with-a-
// literal-string — the moment a file is transformed, regardless of whether
// that code path ever runs. A top-level `import ... from "@api"` in THIS file
// would therefore hard-fail module resolution as soon as vitest loads the
// test file to collect it, even on the skip path, because "the pkg is
// missing" only becomes true information at *runtime* — well after vite has
// already tried (and failed) to resolve the alias. So the `@api` import is
// deferred into each `it()` body (only reached when NOT skipped).
import { describe, expect, it } from "vitest";

if (!__WASM_PKG_PRESENT__) {
  // eslint-disable-next-line no-console
  console.warn(
    "[demo-browser] crates/koji-wasm/pkg is missing — skipping the real-wasm " +
      "smoke test. Run `bun run wasm:build` first, then `bun run test:demo`.",
  );
}

describe.skipIf(!__WASM_PKG_PRESENT__)("demo mode end-to-end (real wasm)", () => {
  it(
    "seeds, lists geofences, and runs a real wasm cluster calc",
    async () => {
      const { ensureSeeded } = await import("@/api/demo/seeds/seed");
      const { getJob, submitCalc, baseDataProvider } = await import("@api");

      // Cross-origin isolation gates the rayon thread pool (SharedArrayBuffer).
      // vitest's browser mode serves this project's files through its own Vite
      // dev server, so the demo-browser project's COOP/COEP headers should
      // apply — but threads are a nice-to-have here, not a requirement: a
      // non-isolated test browser still runs the calc single-threaded. Warn,
      // don't fail.
      if (!window.crossOriginIsolated) {
        // eslint-disable-next-line no-console
        console.warn(
          "[demo-browser] crossOriginIsolated is false — wasm rayon threads are " +
            "unavailable in this test browser; the calc below runs single-threaded.",
        );
      }

      await ensureSeeded();

      const fences = await baseDataProvider.getList("geofence", {
        pagination: { page: 1, perPage: 5 },
        sort: { field: "id", order: "ASC" },
        filter: {},
      });
      expect(fences.total).toBe(32);

      const fence = await baseDataProvider.getOne("geofence", { id: 1 });

      const body = {
        mode: "route",
        category: "spawnpoint",
        area: {
          type: "FeatureCollection",
          features: [{ type: "Feature", properties: {}, geometry: fence.data.geometry }],
        },
        clustering: { calculationMode: "radius", radius: 70, minPoints: 3 },
      };
      const id = await submitCalc(body);

      let rec = await getJob(id);
      for (let i = 0; i < 200 && !["succeeded", "failed"].includes(rec.status); i++) {
        await new Promise((resolve) => setTimeout(resolve, 100));
        rec = await getJob(id);
      }
      expect(rec.status).toBe("succeeded");

      const fc = rec.result?.data as GeoJSON.FeatureCollection;
      expect(fc.type).toBe("FeatureCollection");

      const stats = rec.result?.stats as { total_clusters: number; total_points: number };
      expect(stats.total_points).toBeGreaterThan(100);
      expect(stats.total_clusters).toBeGreaterThan(0);
    },
    // Cold wasm compile + rayon thread-pool spin-up + up to 20s of polling
    // (200 * 100ms) comfortably clears vitest's 5s default; give it headroom.
    45_000,
  );

  it("algorithm options come from wasm", async () => {
    const { getAlgorithms } = await import("@api");
    const o = await getAlgorithms();
    expect(o.clustering).toContain("balanced");
    expect(o.routing).toContain("tsp");
    expect(o.bootstrap).toContain("radius");
  });
});
