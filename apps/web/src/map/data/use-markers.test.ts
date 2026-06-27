import { afterEach, expect, test, vi } from "vitest";
import { fetchMarkers } from "@/map/data/use-markers";
import type { Bounds } from "@/map/stores/types";

afterEach(() => vi.restoreAllMocks());
const B: Bounds = [-122.4, 47.4, -122.2, 47.6];

test("fetchMarkers POSTs the bbox and returns [lat,lon] pairs (enveloped)", async () => {
  const fetchMock = vi.fn().mockResolvedValue(
    new Response(JSON.stringify({ status: "ok", data: { points: [[47.5, -122.3]] } }), { status: 200 }),
  );
  vi.stubGlobal("fetch", fetchMock);

  const pts = await fetchMarkers("pokestop", B, 0);

  const [url, init] = fetchMock.mock.calls[0];
  expect(url).toBe("/api/v2/golbat-data/pokestop");
  expect(init.method).toBe("POST");
  // Real Rust BboxInput uses serde rename_all = "camelCase" → minLat/maxLon on wire.
  // Assert ALL four corners so a transpose/casing regression in any field is caught.
  expect(JSON.parse(init.body).bbox).toEqual({
    minLat: 47.4, minLon: -122.4, maxLat: 47.6, maxLon: -122.2,
  });
  expect(pts).toEqual([[47.5, -122.3]]);
});

test("fetchMarkers also accepts a raw (non-enveloped) {points} body", async () => {
  vi.stubGlobal("fetch", vi.fn().mockResolvedValue(
    new Response(JSON.stringify({ points: [[1, 2]] }), { status: 200 }),
  ));
  expect(await fetchMarkers("gym", B, 0)).toEqual([[1, 2]]);
});

test("fetchMarkers sends tth only for spawnpoint", async () => {
  const makeResp = () =>
    new Response(JSON.stringify({ status: "ok", data: { points: [] } }), { status: 200 });
  const fetchMock = vi.fn().mockImplementation(() => Promise.resolve(makeResp()));
  vi.stubGlobal("fetch", fetchMock);
  await fetchMarkers("spawnpoint", B, 100, "Known");
  expect(JSON.parse(fetchMock.mock.calls[0][1].body)).toMatchObject({ lastSeen: 100, tth: "Known" });
  fetchMock.mockClear();
  await fetchMarkers("gym", B, 100, "Known");
  expect(JSON.parse(fetchMock.mock.calls[0][1].body).tth).toBeUndefined();
});
