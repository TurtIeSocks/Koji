import { describe, expect, it, vi } from "vitest";
import { KojiDrawPolygonMode } from "./koji-draw-polygon-mode";

const EMPTY_FC = { type: "FeatureCollection", features: [] };

// Minimal ModeProps stand-in — only the fields the mode actually touches.
function props(overrides: Record<string, unknown> = {}) {
  return {
    data: EMPTY_FC,
    selectedIndexes: [],
    modeConfig: {},
    onEdit: vi.fn(),
    onUpdateCursor: vi.fn(),
    lastPointerMoveEvent: { mapCoords: [9, 9], picks: [], screenCoords: [90, 90] },
    ...overrides,
  } as never;
}
function click(mapCoords: [number, number], screenCoords: [number, number]) {
  return { mapCoords, screenCoords, picks: [], sourceEvent: {} } as never;
}

describe("KojiDrawPolygonMode", () => {
  it("keeps an open LineString tentative even past 3 vertices (v1 parity)", () => {
    const m = new KojiDrawPolygonMode();
    const p = props();
    m.handleClick(click([0, 0], [0, 0]), p);
    m.handleClick(click([1, 0], [100, 0]), p);
    m.handleClick(click([1, 1], [100, 100]), p);
    const tentative = m.createTentativeFeature(p);
    expect(tentative.geometry.type).toBe("LineString"); // stock mode: "Polygon"
    // clicked points + cursor, NO closing segment back to the start
    expect((tentative.geometry as GeoJSON.LineString).coordinates).toEqual([
      [0, 0], [1, 0], [1, 1], [9, 9],
    ]);
  });

  it("drops the echo click of a double-click (same spot, <300ms)", () => {
    let now = 1000;
    const m = new KojiDrawPolygonMode(() => now);
    const p = props();
    m.handleClick(click([0, 0], [50, 50]), p);
    now += 80; // double-click echo: same pixel, 80ms later
    m.handleClick(click([0, 0], [51, 50]), p);
    expect(m.getClickSequence()).toHaveLength(1);
    now += 1000; // a deliberate later click at the same spot still lands
    m.handleClick(click([0, 0], [51, 50]), p);
    expect(m.getClickSequence()).toHaveLength(2);
  });

  it("finish() emits addFeature once 3+ vertices exist", () => {
    const m = new KojiDrawPolygonMode();
    const p = props();
    m.handleClick(click([0, 0], [0, 0]), p);
    m.handleClick(click([1, 0], [100, 0]), p);
    m.handleClick(click([1, 1], [100, 100]), p);
    (p as { onEdit: ReturnType<typeof vi.fn> }).onEdit.mockClear();
    m.finish();
    const types = (p as { onEdit: ReturnType<typeof vi.fn> }).onEdit.mock.calls.map(
      (c) => c[0].editType,
    );
    expect(types).toContain("addFeature");
    expect(m.getClickSequence()).toHaveLength(0);
  });

  it("cancel() resets the sequence and emits cancelFeature", () => {
    const m = new KojiDrawPolygonMode();
    const p = props();
    m.handleClick(click([0, 0], [0, 0]), p);
    m.cancel();
    expect(m.getClickSequence()).toHaveLength(0);
    const types = (p as { onEdit: ReturnType<typeof vi.fn> }).onEdit.mock.calls.map(
      (c) => c[0].editType,
    );
    expect(types).toContain("cancelFeature");
  });

  it("finish() with <3 vertices behaves as cancel", () => {
    const m = new KojiDrawPolygonMode();
    const p = props();
    m.handleClick(click([0, 0], [0, 0]), p);
    m.finish();
    expect(m.getClickSequence()).toHaveLength(0);
    const types = (p as { onEdit: ReturnType<typeof vi.fn> }).onEdit.mock.calls.map(
      (c) => c[0].editType,
    );
    expect(types).not.toContain("addFeature");
  });
});
