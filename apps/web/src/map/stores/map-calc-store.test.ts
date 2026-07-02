import { beforeEach, expect, test } from "vitest";
import { useMapCalcStore } from "@/map/stores/map-calc-store";

beforeEach(() => useMapCalcStore.setState(useMapCalcStore.getInitialState()));

test("startJob tracks the job and clears any prior result/error", () => {
  useMapCalcStore.setState({ resultFC: { type: "FeatureCollection", features: [] }, error: "old" });
  useMapCalcStore.getState().startJob("42");
  const s = useMapCalcStore.getState();
  expect(s.job).toEqual({ id: "42", status: "queued", progress: 0, phase: null });
  expect(s.resultFC).toBeNull();
  expect(s.error).toBeNull();
});

test("updateJob merges progress/phase and is a no-op without a job", () => {
  useMapCalcStore.getState().updateJob({ status: "running", progress: 0.5, phase: "clustering" });
  expect(useMapCalcStore.getState().job).toBeNull(); // no job yet → ignored

  useMapCalcStore.getState().startJob("1");
  useMapCalcStore.getState().updateJob({ status: "running", progress: 0.5, phase: "clustering" });
  expect(useMapCalcStore.getState().job).toMatchObject({ status: "running", progress: 0.5, phase: "clustering" });
  // phase omitted → keeps the last phase
  useMapCalcStore.getState().updateJob({ status: "running", progress: 0.8 });
  expect(useMapCalcStore.getState().job?.phase).toBe("clustering");
});

test("setResult marks succeeded; setError marks failed", () => {
  const fc: GeoJSON.FeatureCollection = { type: "FeatureCollection", features: [] };
  useMapCalcStore.getState().startJob("1");
  useMapCalcStore.getState().setResult(fc, { total_clusters: 3 });
  expect(useMapCalcStore.getState().job?.status).toBe("succeeded");
  expect(useMapCalcStore.getState().resultFC).toBe(fc);

  useMapCalcStore.getState().startJob("2");
  useMapCalcStore.getState().setError("boom");
  expect(useMapCalcStore.getState().job?.status).toBe("failed");
  expect(useMapCalcStore.getState().error).toBe("boom");
});
