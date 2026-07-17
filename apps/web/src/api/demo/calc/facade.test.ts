import { describe, expect, it, vi } from "vitest";
import { demoBus } from "../realtime";
import { createCalcFacade } from "./facade";

const EMPTY_FC: GeoJSON.FeatureCollection = { type: "FeatureCollection", features: [] };

describe("createCalcFacade", () => {
  it("lifecycle: submit → running event → terminal event + getJob result", async () => {
    const events: string[] = [];
    const fake = async () => ({
      ok: true as const,
      result: { data: { type: "FeatureCollection", features: [] }, stats: { total_clusters: 1 } },
    });
    const { submitCalc, getJob } = createCalcFacade({ postCalc: fake, resolvePoints: async () => [[40, -74]] });
    demoBus.subscribe("jobs/demo-1", (e) => events.push((e.payload as { status: string }).status));
    const id = await submitCalc({ mode: "route", area: EMPTY_FC, clustering: { radius: 70 } });
    expect(id).toBe("demo-1");
    await vi.waitFor(async () => {
      const rec = await getJob(id);
      expect(rec.status).toBe("succeeded");
    });
    expect(events).toContain("running");
    expect(events).toContain("succeeded");
    expect((await getJob(id)).result?.stats).toBeTruthy();
  });

  it("worker failure → failed record with error", async () => {
    const fake = async () => ({ ok: false as const, error: "boom" });
    const { submitCalc, getJob } = createCalcFacade({ postCalc: fake, resolvePoints: async () => [] });
    const id = await submitCalc({ mode: "route", area: EMPTY_FC });
    await vi.waitFor(async () => expect((await getJob(id)).status).toBe("failed"));
    expect((await getJob(id)).error).toBe("boom");
  });

  it("injects the resolved dataPoints into the body handed to postCalc", async () => {
    let seen: Record<string, unknown> | null = null;
    const fake = async (body: Record<string, unknown>) => {
      seen = body;
      return { ok: true as const, result: { data: null, stats: {} } };
    };
    const { submitCalc, getJob } = createCalcFacade({
      postCalc: fake,
      resolvePoints: async () => [
        [1, 2],
        [3, 4],
      ],
    });
    const id = await submitCalc({ mode: "route", area: EMPTY_FC });
    await vi.waitFor(async () => expect((await getJob(id)).status).toBe("succeeded"));
    expect(seen).not.toBeNull();
    expect((seen! as { dataPoints?: unknown }).dataPoints).toEqual([
      [1, 2],
      [3, 4],
    ]);
  });

  it("getJob on an unknown id throws (live 404 parity)", async () => {
    const { getJob } = createCalcFacade({
      postCalc: async () => ({ ok: true as const, result: { data: null, stats: {} } }),
      resolvePoints: async () => [],
    });
    await expect(getJob("demo-999")).rejects.toThrow();
  });

  it("getAlgorithms caches the options after the first resolve", async () => {
    let calls = 0;
    const getOptions = async () => {
      calls += 1;
      return { clustering: ["balanced"], routing: ["tsp"], bootstrap: ["radius"] };
    };
    const { getAlgorithms } = createCalcFacade({
      postCalc: async () => ({ ok: true as const, result: { data: null, stats: {} } }),
      resolvePoints: async () => [],
      getOptions,
    });
    const a = await getAlgorithms();
    const b = await getAlgorithms();
    expect(a).toBe(b);
    expect(calls).toBe(1);
  });
});
