import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { describe, expect, it, vi } from "vitest";
import { DEFAULT_TILE_URL } from "@/lib/constants";

(
  globalThis as unknown as { IS_REACT_ACT_ENVIRONMENT?: boolean }
).IS_REACT_ACT_ENVIRONMENT = true;

// A plain delegate, not a vi.fn: vitest's spy bookkeeping re-surfaces a
// rejected mock promise as an unhandled rejection even though the code under
// test catches it.
const config = vi.hoisted(() => ({
  load: (): Promise<unknown> => Promise.resolve({}),
}));
vi.mock("@api", () => ({ loadConfig: () => config.load() }));

// Hand-rolled like use-marker-overlay.test.tsx: @testing-library/react isn't a dependency.
function renderHook<T>(callback: () => T) {
  const result: { current: T } = { current: undefined as unknown as T };
  function Probe() {
    result.current = callback();
    return null;
  }
  const container = document.createElement("div");
  let root: Root;
  act(() => {
    root = createRoot(container);
    root.render(<Probe />);
  });
  return { result, unmount: () => act(() => root.unmount()) };
}

const settle = () => act(async () => {});

// app-config caches the /internal/config fetch at module level, so each test
// loads a fresh copy of the hook.
async function freshUseTileUrl() {
  vi.resetModules();
  return (await import("./use-tile-url")).useTileUrl;
}

describe("useTileUrl", () => {
  it("falls back to the OSM default when config sets no tile server", async () => {
    config.load = () => Promise.resolve({ tile_server: "" });
    const useTileUrl = await freshUseTileUrl();
    const { result, unmount } = renderHook(() => useTileUrl());
    await settle();
    expect(result.current).toBe("https://tile.openstreetmap.org/{z}/{x}/{y}.png");
    expect(result.current).toBe(DEFAULT_TILE_URL);
    unmount();
  });

  it("uses the tile server set in Koji's config over the default", async () => {
    config.load = () => Promise.resolve({ tile_server: "https://tiles.example/{z}/{x}/{y}.png" });
    const useTileUrl = await freshUseTileUrl();
    const { result, unmount } = renderHook(() => useTileUrl());
    await settle();
    expect(result.current).toBe("https://tiles.example/{z}/{x}/{y}.png");
    unmount();
  });

  it("keeps the default when the config fetch fails", async () => {
    config.load = () => Promise.reject(new Error("401"));
    const useTileUrl = await freshUseTileUrl();
    const { result, unmount } = renderHook(() => useTileUrl());
    await settle();
    expect(result.current).toBe(DEFAULT_TILE_URL);
    unmount();
  });

  it("lets an explicit tile url (the tile server resource) win over config", async () => {
    config.load = () => Promise.resolve({ tile_server: "https://tiles.example/{z}/{x}/{y}.png" });
    const useTileUrl = await freshUseTileUrl();
    const { result, unmount } = renderHook(() => useTileUrl("https://resource.example/{z}/{x}/{y}.png"));
    await settle();
    expect(result.current).toBe("https://resource.example/{z}/{x}/{y}.png");
    unmount();
  });
});
