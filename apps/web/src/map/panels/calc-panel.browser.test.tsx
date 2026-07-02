import { beforeEach, expect, test, vi } from "vitest";
import { render } from "vitest-browser-react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { CalcPanel } from "@/map/panels/calc-panel";
import { useMapCalcStore } from "@/map/stores/map-calc-store";
import { useMapUIStore } from "@/map/stores/map-ui-store";

const { submitCalcMock, getAlgorithmsMock } = vi.hoisted(() => ({
  submitCalcMock: vi.fn(),
  getAlgorithmsMock: vi.fn(),
}));
vi.mock("@/map/data/calc-client", () => ({
  submitCalc: submitCalcMock,
  getAlgorithms: getAlgorithmsMock,
}));

function renderPanel() {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={qc}>
      <CalcPanel />
    </QueryClientProvider>,
  );
}

beforeEach(() => {
  useMapCalcStore.setState(useMapCalcStore.getInitialState());
  useMapUIStore.setState(useMapUIStore.getInitialState());
  submitCalcMock.mockReset();
  submitCalcMock.mockResolvedValue("9");
  getAlgorithmsMock.mockReset();
  getAlgorithmsMock.mockResolvedValue({ clustering: ["fastest"], routing: [], bootstrap: [] });
});

test("area modes are gated until a geofence is selected, then calc over it", async () => {
  const geom: GeoJSON.Polygon = { type: "Polygon", coordinates: [[[0, 0], [1, 0], [1, 1], [0, 0]]] };
  const screen = renderPanel();
  // Default cluster mode, no geofence → blocked with the hint.
  await expect.element(screen.getByRole("button", { name: /calculate/i })).toBeDisabled();
  await expect.element(screen.getByText(/select a geofence first/i)).toBeInTheDocument();

  // Clicking a geofence loads it into the editor (top-level feature.id) → unblocks.
  useMapUIStore.getState().editFeature({ type: "Feature", id: 3, properties: {}, geometry: geom });
  const btn = screen.getByRole("button", { name: /calculate/i });
  await expect.element(btn).toBeEnabled();
  await btn.click();

  await vi.waitFor(() => expect(submitCalcMock).toHaveBeenCalledTimes(1));
  const body = submitCalcMock.mock.calls[0][0] as { mode: string; area: GeoJSON.FeatureCollection };
  expect(body.mode).toBe("cluster");
  expect(body.area.features[0].geometry).toEqual(geom); // the selected geofence
  await vi.waitFor(() => expect(useMapCalcStore.getState().job?.id).toBe("9"));
});

test("Reroute is blocked until a route is selected", async () => {
  useMapCalcStore.setState({ mode: "reroute" });
  const screen = renderPanel();
  await expect.element(screen.getByRole("button", { name: /calculate/i })).toBeDisabled();
  await expect.element(screen.getByText(/select a route first/i)).toBeInTheDocument();

  // Selecting a route unblocks it.
  useMapUIStore.setState({ selection: { kind: "route", id: "1" } });
  await expect.element(screen.getByRole("button", { name: /calculate/i })).toBeEnabled();
});
