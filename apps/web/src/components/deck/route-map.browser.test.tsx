import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { render } from "vitest-browser-react";
import { useWatch } from "react-hook-form";
import { testDataProvider, type DataProvider } from "shadmin-core";
import { AdminContext, SimpleForm } from "@/components/admin";
import type { UseCalcReturn } from "./use-calc";

// `useCalc` (Task 9) is exercised by its own unit tests — here we control its
// return value directly to drive the "succeeded result -> geometry" effect
// without needing a real calc-client round trip.
const { calcMock, useCalcSpy } = vi.hoisted(() => {
  const calcMock: import("./use-calc").UseCalcReturn = {
    params: { mode: "cluster", strategy: "radius", radius: 70, s2Level: 15, s2Size: 9, minPoints: 3, clusterMode: null, maxClusters: null, centerClusters: false, sortBy: null, tth: "All" },
    setParams: vi.fn(),
    job: null,
    result: null,
    stats: null,
    error: null,
    run: vi.fn(async () => {}),
    clear: vi.fn(),
  };
  return { calcMock, useCalcSpy: vi.fn(() => calcMock) };
});
vi.mock("./use-calc", () => ({ useCalc: useCalcSpy }));
vi.mock("@/map/data/use-markers", () => ({ useMarkers: vi.fn(() => ({ data: [] })) }));

import { useMarkers } from "@/map/data/use-markers";
import { RouteMap } from "./route-map";

const fencePoly: GeoJSON.Polygon = {
  type: "Polygon",
  coordinates: [[[0, 0], [1, 0], [1, 1], [0, 1], [0, 0]]],
};

function GeometryProbe() {
  const geometry = useWatch({ name: "geometry" });
  const mode = useWatch({ name: "mode" });
  return (
    <div data-testid="geometry-probe" data-mode={typeof mode === "string" ? mode : ""}>
      {JSON.stringify(geometry)}
    </div>
  );
}

function renderRouteMap(record: Record<string, unknown> = { geofence_id: 7, geometry: null, mode: "unset" }) {
  const getOneSpy: DataProvider["getOne"] = vi.fn(async (_resource, params) => ({
    data: { id: params.id, geometry: fencePoly },
  })) as DataProvider["getOne"];
  const screen = render(
    <AdminContext dataProvider={testDataProvider({ getOne: getOneSpy })}>
      <SimpleForm onSubmit={() => {}} record={record}>
        <RouteMap />
        <GeometryProbe />
      </SimpleForm>
    </AdminContext>,
  );
  return { screen, getOneSpy };
}

function resetCalc(overrides: Partial<UseCalcReturn> = {}) {
  Object.assign(calcMock, {
    params: { mode: "cluster", strategy: "radius", radius: 70, s2Level: 15, s2Size: 9, minPoints: 3, clusterMode: null, maxClusters: null, centerClusters: false, sortBy: null, tth: "All" },
    job: null,
    result: null,
    stats: null,
    error: null,
    ...overrides,
  });
}

beforeEach(() => resetCalc());
afterEach(() => useCalcSpy.mockClear());

describe("RouteMap", () => {
  it("fetches the parent fence by geofence_id and renders the map", async () => {
    const { screen, getOneSpy } = renderRouteMap();
    await expect.element(screen.getByTestId("deck-map")).toBeInTheDocument();
    await vi.waitFor(() => {
      expect(getOneSpy).toHaveBeenCalledWith("geofence", expect.objectContaining({ id: 7 }));
    });
  });

  it("renders the last-seen date picker", async () => {
    const { screen } = renderRouteMap();
    await expect.element(screen.getByLabelText("Last seen after")).toBeInTheDocument();
  });

  it("a succeeded calc result becomes the route geometry (MultiPoint, no transpose)", async () => {
    resetCalc({
      result: {
        type: "FeatureCollection",
        features: [
          { type: "Feature", geometry: { type: "MultiPoint", coordinates: [[10, 20], [11, 21]] }, properties: {} },
        ],
      },
    });
    const { screen } = renderRouteMap();
    await expect.element(screen.getByTestId("geometry-probe")).toHaveTextContent(
      JSON.stringify({ type: "MultiPoint", coordinates: [[10, 20], [11, 21]] }),
    );
  });

  it("derives spawnpoint from route mode 'pokemon' (offers the spawnpoint-only Tth)", async () => {
    // route.mode "pokemon" → category spawnpoint; the panel then shows the
    // spawnpoint-only Tth control (the category itself isn't rendered).
    const { screen } = renderRouteMap({ geofence_id: 7, geometry: null, mode: "pokemon" });
    await expect.element(screen.getByRole("combobox", { name: "Tth" })).toBeInTheDocument();
  });

  it("previews the mode's markers scoped to the fence (fort → gyms)", async () => {
    const markersMock = vi.mocked(useMarkers);
    renderRouteMap({ geofence_id: 7, geometry: null, mode: "fort" });
    // fort shows gym + station + pokestop; each is fetched with the fence polygon
    // as the area and enabled once the fence loads.
    await vi.waitFor(() => {
      expect(markersMock).toHaveBeenCalledWith("gym", fencePoly, expect.anything(), expect.any(Number), true);
    });
  });

  it("shows no markers until a geofence is selected (all fetches disabled)", async () => {
    const markersMock = vi.mocked(useMarkers);
    markersMock.mockClear();
    renderRouteMap({ geofence_id: null, geometry: null, mode: "fort" });
    await vi.waitFor(() => expect(markersMock).toHaveBeenCalled());
    // Every useMarkers call has enabled=false (5th arg) — no geofence, no data.
    expect(markersMock.mock.calls.every((c) => c[4] === false)).toBe(true);
  });

  it("a succeeded result does NOT overwrite the user-chosen route mode", async () => {
    resetCalc({
      result: {
        type: "FeatureCollection",
        features: [{ type: "Feature", geometry: { type: "MultiPoint", coordinates: [[1, 2]] }, properties: {} }],
      },
    });
    const { screen } = renderRouteMap({ geofence_id: 7, geometry: null, mode: "quest" });
    // geometry is written, but mode stays what the user set (was auto-set before).
    await expect.element(screen.getByTestId("geometry-probe")).toHaveAttribute("data-mode", "quest");
  });
});
