import { afterEach, expect, test, vi } from "vitest";
import { fetchFeatureCollection } from "@/map/data/use-geo-features";

afterEach(() => vi.restoreAllMocks());

test("fetchFeatureCollection requests featurecollection format and returns it", async () => {
  const fc = { type: "FeatureCollection", features: [] };
  const fetchMock = vi.fn().mockResolvedValue(
    new Response(JSON.stringify({ status: "ok", data: fc }), { status: 200 }),
  );
  vi.stubGlobal("fetch", fetchMock);
  const out = await fetchFeatureCollection("geofences");
  expect(fetchMock.mock.calls[0][0]).toBe("/api/v2/geofences?format=featurecollection");
  expect(out).toEqual(fc);
});

test("fetchFeatureCollection accepts a raw (non-enveloped) FeatureCollection", async () => {
  const fc = { type: "FeatureCollection", features: [] };
  const fetchMock = vi.fn().mockResolvedValue(
    new Response(JSON.stringify(fc), { status: 200 }),
  );
  vi.stubGlobal("fetch", fetchMock);
  const out = await fetchFeatureCollection("routes");
  expect(fetchMock.mock.calls[0][0]).toBe("/api/v2/routes?format=featurecollection");
  expect(out).toEqual(fc);
});
