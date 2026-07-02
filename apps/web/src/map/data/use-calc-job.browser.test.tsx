import { beforeEach, expect, test, vi } from "vitest";
import { render } from "vitest-browser-react";
import { useMapCalcStore } from "@/map/stores/map-calc-store";
import { useCalcJob } from "@/map/data/use-calc-job";

const { subscribeMock, getJobMock } = vi.hoisted(() => ({
  subscribeMock: vi.fn(),
  getJobMock: vi.fn(),
}));

vi.mock("@/components/realtime", () => ({
  useSubscribe: (topic: string, cb: unknown, opts?: unknown) => subscribeMock(topic, cb, opts),
}));
vi.mock("@/map/data/calc-client", () => ({ getJob: getJobMock }));

function Probe() {
  useCalcJob();
  return null;
}

beforeEach(() => {
  useMapCalcStore.setState(useMapCalcStore.getInitialState());
  subscribeMock.mockReset();
  getJobMock.mockReset();
});

test("subscribes to jobs/{id} and resolves an already-succeeded job on mount", async () => {
  const fc: GeoJSON.FeatureCollection = { type: "FeatureCollection", features: [] };
  getJobMock.mockResolvedValue({
    id: 5,
    status: "succeeded",
    progress: 1,
    phase: "done",
    result: { data: fc, stats: { total_clusters: 2 } },
  });
  useMapCalcStore.getState().startJob("5");
  render(<Probe />);

  await vi.waitFor(() =>
    expect(subscribeMock).toHaveBeenCalledWith("jobs/5", expect.any(Function), expect.anything()),
  );
  await vi.waitFor(() => expect(useMapCalcStore.getState().resultFC).toBe(fc));
  expect(useMapCalcStore.getState().job?.status).toBe("succeeded");
});

test("a realtime running event updates live progress + phase", async () => {
  getJobMock.mockResolvedValue({ id: 6, status: "running", progress: 0.2, phase: "clustering" });
  useMapCalcStore.getState().startJob("6");
  render(<Probe />);
  await vi.waitFor(() => expect(subscribeMock).toHaveBeenCalled());

  const cb = subscribeMock.mock.calls[0][1] as (e: { type: string; payload: unknown }) => void;
  cb({ type: "updated", payload: { id: 6, status: "running", progress: 0.6, phase: "routing" } });

  await vi.waitFor(() => expect(useMapCalcStore.getState().job?.progress).toBe(0.6));
  expect(useMapCalcStore.getState().job?.phase).toBe("routing");
});
