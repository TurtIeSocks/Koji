import { afterEach, expect, test, vi } from "vitest";
import { fetchMarkers } from "@/api/live/markers";
import type { Bounds } from "@/map/stores/types";

afterEach(() => vi.restoreAllMocks());
const B: Bounds = [-122.4, 47.4, -122.2, 47.6];

test("fetchMarkers POSTs the bbox and returns [lat,lon] pairs (enveloped)", async () => {
  const fetchMock = vi.fn().mockResolvedValue(
    new Response(JSON.stringify({ status: "ok", data: { points: [[47.5, -122.3]] } }), { status: 200 }),
  );
  vi.stubGlobal("fetch", fetchMock);

  // area=null → the bbox fallback path.
  const pts = await fetchMarkers("pokestop", null, B, 0);

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

test("fetchMarkers sends the polygon `area` (not bbox) when a geometry is given", async () => {
  const poly: GeoJSON.Polygon = {
    type: "Polygon",
    coordinates: [[[0, 0], [1, 0], [1, 1], [0, 1], [0, 0]]],
  };
  const fetchMock = vi.fn().mockResolvedValue(
    new Response(JSON.stringify({ status: "ok", data: { points: [] } }), { status: 200 }),
  );
  vi.stubGlobal("fetch", fetchMock);

  await fetchMarkers("pokestop", poly, B, 0);

  const body = JSON.parse(fetchMock.mock.calls[0][1].body);
  // The real shape rides as `area`; `bbox` must NOT be sent (it would over-return
  // points in the gaps of a MultiPolygon).
  expect(body.area).toEqual(poly);
  expect(body.bbox).toBeUndefined();
});

test("fetchMarkers also accepts a raw (non-enveloped) {points} body", async () => {
  vi.stubGlobal("fetch", vi.fn().mockResolvedValue(
    new Response(JSON.stringify({ points: [[1, 2]] }), { status: 200 }),
  ));
  expect(await fetchMarkers("gym", null, B, 0)).toEqual([[1, 2]]);
});

test("fetchMarkers sends tth only for spawnpoint", async () => {
  const makeResp = () =>
    new Response(JSON.stringify({ status: "ok", data: { points: [] } }), { status: 200 });
  const fetchMock = vi.fn().mockImplementation(() => Promise.resolve(makeResp()));
  vi.stubGlobal("fetch", fetchMock);
  await fetchMarkers("spawnpoint", null, B, 100, "Known");
  expect(JSON.parse(fetchMock.mock.calls[0][1].body)).toMatchObject({ lastSeen: 100, tth: "Known" });
  fetchMock.mockClear();
  await fetchMarkers("gym", null, B, 100, "Known");
  expect(JSON.parse(fetchMock.mock.calls[0][1].body).tth).toBeUndefined();
});
