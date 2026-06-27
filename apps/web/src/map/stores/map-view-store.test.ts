import { beforeEach, afterEach, expect, test, vi } from "vitest";
import { createMapViewStore } from "@/map/stores/map-view-store";
import type { ViewState, Bounds } from "@/map/stores/types";

const VS: ViewState = { longitude: 1, latitude: 2, zoom: 10, pitch: 0, bearing: 0 };
const B: Bounds = [0, 0, 2, 4];

beforeEach(() => vi.useFakeTimers());
afterEach(() => vi.useRealTimers());

test("setLive updates liveViewState immediately but NOT settled (until throttle fires)", () => {
  const store = createMapViewStore(200);
  store.getState().setLive(VS, B);
  expect(store.getState().liveViewState).toEqual(VS);
  // settled still at defaults right after a single setLive
  expect(store.getState().settledBounds).not.toEqual(B);
  vi.advanceTimersByTime(200);
  expect(store.getState().settledViewState).toEqual(VS);
  expect(store.getState().settledBounds).toEqual(B);
});

test("rapid setLive calls notify settled subscribers at most once per window", () => {
  const store = createMapViewStore(200);
  const settledSpy = vi.fn();
  // subscribe ONLY to settledBounds (subscribeWithSelector)
  store.subscribe((s) => s.settledBounds, settledSpy);
  for (let i = 0; i < 10; i++) store.getState().setLive({ ...VS, zoom: 10 + i }, B);
  expect(settledSpy).not.toHaveBeenCalled(); // throttled, not yet flushed
  vi.advanceTimersByTime(200);
  expect(settledSpy).toHaveBeenCalledTimes(1);
  // coalesces to the LAST pending value, not the first/intermediate
  expect(store.getState().settledViewState.zoom).toBe(19);
});
