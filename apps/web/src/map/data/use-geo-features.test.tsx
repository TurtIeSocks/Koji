import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, describe, expect, it, test, vi } from "vitest";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { fetchFeatureCollection, useGeofencesByBbox, useGeofencesByIds } from "@/map/data/use-geo-features";
import type { Bounds } from "@/map/stores/types";

// Silence React's "not configured to support act(...)" warning — normally set
// by `@testing-library/react`'s own environment setup, which this project
// doesn't depend on (see the hand-rolled `renderHook` below).
(
  globalThis as unknown as { IS_REACT_ACT_ENVIRONMENT?: boolean }
).IS_REACT_ACT_ENVIRONMENT = true;

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

// `@testing-library/react` isn't a dependency of this project — hand-roll the
// minimal `renderHook` shape (result ref + act) backed by `react-dom/client`
// + React 19's own `act`, matching `use-calc.test.tsx` / `use-deck-edit-rhf.test.tsx`.
// Wrapped in a fresh `QueryClientProvider` (retries off) per call, since these
// two hooks are react-query hooks.
function renderHook<T>(callback: () => T) {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  const result: { current: T } = { current: undefined as unknown as T };
  function Probe() {
    result.current = callback();
    return null;
  }
  const container = document.createElement("div");
  document.body.appendChild(container);
  let root: Root;
  act(() => {
    root = createRoot(container);
    root.render(
      <QueryClientProvider client={qc}>
        <Probe />
      </QueryClientProvider>,
    );
  });
  return {
    result,
    unmount: () => act(() => root.unmount()),
  };
}

let mounted: { unmount: () => void } | null = null;
afterEach(() => {
  mounted?.unmount();
  mounted = null;
});

const twoFeatureFc: GeoJSON.FeatureCollection = {
  type: "FeatureCollection",
  features: [
    { type: "Feature", geometry: { type: "Point", coordinates: [0, 0] }, properties: { id: 1 } },
    { type: "Feature", geometry: { type: "Point", coordinates: [1, 1] }, properties: { id: 2 } },
  ],
};

function stubFetchOk(fc: GeoJSON.FeatureCollection) {
  const fetchMock = vi.fn().mockResolvedValue(
    new Response(JSON.stringify({ status: "ok", data: fc }), { status: 200 }),
  );
  vi.stubGlobal("fetch", fetchMock);
  return fetchMock;
}

describe("useGeofencesByIds", () => {
  it("fetches by ids and resolves the mocked FeatureCollection", async () => {
    const fetchMock = stubFetchOk(twoFeatureFc);

    const rendered = renderHook(() => useGeofencesByIds([2, 1]));
    mounted = rendered;
    const { result } = rendered;

    await act(async () => {
      await vi.waitFor(() => expect(result.current.isSuccess).toBe(true));
    });

    expect(fetchMock).toHaveBeenCalledTimes(1);
    const url = fetchMock.mock.calls[0][0] as string;
    expect(url).toContain("/api/v2/geofences?format=featurecollection&ids=");
    expect(url).toContain("1");
    expect(url).toContain("2");
    expect(result.current.data).toEqual(twoFeatureFc);
  });

  it("does not fetch when ids is empty (disabled)", async () => {
    const fetchMock = stubFetchOk(twoFeatureFc);

    const rendered = renderHook(() => useGeofencesByIds([]));
    mounted = rendered;

    await act(async () => {});
    expect(fetchMock).not.toHaveBeenCalled();
  });
});

describe("useGeofencesByBbox", () => {
  it("fetches by bbox and resolves the mocked FeatureCollection", async () => {
    const fetchMock = stubFetchOk(twoFeatureFc);
    const bbox: Bounds = [0, 0, 1, 1];

    const rendered = renderHook(() => useGeofencesByBbox(bbox, true));
    mounted = rendered;
    const { result } = rendered;

    await act(async () => {
      await vi.waitFor(() => expect(result.current.isSuccess).toBe(true));
    });

    expect(fetchMock).toHaveBeenCalledTimes(1);
    expect(fetchMock.mock.calls[0][0]).toBe(
      "/api/v2/geofences?format=featurecollection&bbox=0,0,1,1",
    );
    expect(result.current.data).toEqual(twoFeatureFc);
  });

  it("does not fetch when bbox is null (disabled)", async () => {
    const fetchMock = stubFetchOk(twoFeatureFc);

    const rendered = renderHook(() => useGeofencesByBbox(null, true));
    mounted = rendered;

    await act(async () => {});
    expect(fetchMock).not.toHaveBeenCalled();
  });

  it("does not fetch when enabled is false", async () => {
    const fetchMock = stubFetchOk(twoFeatureFc);

    const rendered = renderHook(() => useGeofencesByBbox([0, 0, 1, 1], false));
    mounted = rendered;

    await act(async () => {});
    expect(fetchMock).not.toHaveBeenCalled();
  });
});
