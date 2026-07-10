import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { loadCamera, saveCamera } from "./map-camera-storage";

const KEY = "koji.map.playground.camera";

describe("map-camera-storage", () => {
  beforeEach(() => {
    localStorage.clear();
    vi.useFakeTimers();
  });
  afterEach(() => {
    vi.runOnlyPendingTimers();
    vi.useRealTimers();
  });

  it("returns null when nothing is stored", () => {
    expect(loadCamera()).toBeNull();
  });

  it("debounces writes and reads back the latest camera", () => {
    saveCamera({ longitude: 1, latitude: 2, zoom: 3 });
    saveCamera({ longitude: 4, latitude: 5, zoom: 6 }); // coalesced into one write
    expect(loadCamera()).toBeNull(); // debounced — not written yet
    vi.advanceTimersByTime(300);
    expect(loadCamera()).toEqual({ longitude: 4, latitude: 5, zoom: 6 });
  });

  it("ignores malformed stored JSON", () => {
    localStorage.setItem(KEY, "{not json");
    expect(loadCamera()).toBeNull();
  });

  it("ignores a partial camera object (missing fields)", () => {
    localStorage.setItem(KEY, JSON.stringify({ longitude: 1 }));
    expect(loadCamera()).toBeNull();
  });
});
