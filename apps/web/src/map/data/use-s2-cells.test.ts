import { afterEach, expect, test, vi } from "vitest";
import { fetchS2Cells } from "@/map/data/use-s2-cells";
import type { Bounds } from "@/map/stores/types";

afterEach(() => vi.restoreAllMocks());
const B: Bounds = [-122.4, 47.4, -122.2, 47.6];

test("fetchS2Cells POSTs /s2/{level} with bbox and returns cells with [lng,lat] rings", async () => {
  // Server sends id + a [lat,lon] corner ring; we transpose to [lng,lat].
  const fetchMock = vi.fn().mockResolvedValue(
    new Response(
      JSON.stringify({ status: "ok", data: [{ id: "abc", coords: [[47.5, -122.3], [47.6, -122.2]] }] }),
      { status: 200 },
    ),
  );
  vi.stubGlobal("fetch", fetchMock);
  const cells = await fetchS2Cells(15, B);
  expect(fetchMock.mock.calls[0][0]).toBe("/api/v2/s2/15");
  expect(JSON.parse(fetchMock.mock.calls[0][1].body)).toMatchObject({ min_lat: 47.4 });
  expect(cells).toEqual([{ id: "abc", ring: [[-122.3, 47.5], [-122.2, 47.6]] }]);
});
