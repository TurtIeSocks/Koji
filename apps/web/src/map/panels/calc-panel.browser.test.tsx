import { beforeEach, expect, test, vi } from "vitest";
import { render } from "vitest-browser-react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { CalcPanel } from "@/map/panels/calc-panel";
import { useMapCalcStore } from "@/map/stores/map-calc-store";
import { useMapUIStore } from "@/map/stores/map-ui-store";
import { useMapViewStore } from "@/map/stores/map-view-store";

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
  useMapViewStore.setState({ settledBounds: [-1, -1, 1, 1] });
  submitCalcMock.mockReset();
  submitCalcMock.mockResolvedValue("9");
  getAlgorithmsMock.mockReset();
  getAlgorithmsMock.mockResolvedValue({ clustering: ["fastest"], routing: [], bootstrap: [] });
});

test("Calculate submits a cluster job over the viewport and tracks it", async () => {
  const screen = renderPanel();
  await screen.getByRole("button", { name: /calculate/i }).click();

  await vi.waitFor(() => expect(submitCalcMock).toHaveBeenCalledTimes(1));
  const body = submitCalcMock.mock.calls[0][0] as Record<string, unknown>;
  expect(body.mode).toBe("cluster");
  expect(body.category).toBe("pokestop");
  expect(body).toHaveProperty("area"); // viewport → area FC
  // Job is now tracked in the store.
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
