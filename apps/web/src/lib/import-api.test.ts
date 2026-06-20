import { beforeEach, describe, expect, it, vi } from "vitest";
import * as http from "@/lib/http";
import { postConvert, postImport } from "./import-api";

describe("postImport", () => {
  beforeEach(() => vi.restoreAllMocks());

  it("POSTs the body to /import and unwraps the result", async () => {
    const fake = {
      committed: true,
      summary: { create: 1, update: 0, skip: 0, fail: 0 },
      results: [{ index: 0, name: "A", action: "create", id: 7, reason: null }],
    };
    const spy = vi
      .spyOn(http, "internalFetch")
      .mockResolvedValue({ status: 200, json: { status: "ok", data: fake } });
    const out = await postImport({ dry_run: false, items: [] });
    expect(spy).toHaveBeenCalledWith(
      "/import",
      expect.objectContaining({ method: "POST" }),
    );
    expect(out.committed).toBe(true);
    expect(out.summary.create).toBe(1);
    expect(out.results[0].action).toBe("create");
  });
});

describe("postConvert", () => {
  beforeEach(() => vi.restoreAllMocks());

  it("POSTs the FeatureCollection to /geometry/convert and returns its features", async () => {
    const fc = {
      type: "FeatureCollection",
      features: [
        { type: "Feature", geometry: { type: "Polygon", coordinates: [] }, properties: { name: "A" } },
      ],
    };
    const spy = vi
      .spyOn(http, "internalFetch")
      .mockResolvedValue({ status: 200, json: { status: "ok", data: fc } });
    const out = await postConvert(fc.features);
    expect(spy).toHaveBeenCalledWith(
      "/geometry/convert",
      expect.objectContaining({ method: "POST" }),
    );
    expect(out).toHaveLength(1);
  });
});
