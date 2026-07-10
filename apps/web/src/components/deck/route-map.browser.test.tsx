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
    params: { mode: "cluster", category: "pokestop", radius: 70, minPoints: 1, clusterMode: null, sortBy: null },
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
    params: { mode: "cluster", category: "pokestop", radius: 70, minPoints: 1, clusterMode: null, sortBy: null },
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

  it("an AREA-mode result also sets the route mode via categoryToRouteMode", async () => {
    resetCalc({
      params: { mode: "cluster", category: "pokestop", radius: 70, minPoints: 1, clusterMode: null, sortBy: null },
      result: {
        type: "FeatureCollection",
        features: [{ type: "Feature", geometry: { type: "MultiPoint", coordinates: [[1, 2]] }, properties: {} }],
      },
    });
    const { screen } = renderRouteMap();
    await expect.element(screen.getByTestId("geometry-probe")).toHaveAttribute("data-mode", "quest");
  });
});
