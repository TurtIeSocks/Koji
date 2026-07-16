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

  it("syncDraft keeps finish() from resurrecting a stale draft (toolbar undo without a canvas event)", () => {
    // Reproduces the Task 10 review finding: draw shape A (committed) → start
    // shape B (3 clicks, all against props whose `data` still has A) → Undo
    // removes A from the RHF draft (no canvas event, so the mode's stash never
    // sees it — buildEditLayer calls syncDraft on every rebuild instead) →
    // without syncDraft, finish() would build updatedData from the STALE
    // pre-undo `data` and resurrect A alongside the new polygon.
    const m = new KojiDrawPolygonMode();
    const fcV1: GeoJSON.FeatureCollection = {
      type: "FeatureCollection",
      features: [
        { type: "Feature", properties: {}, geometry: { type: "Point", coordinates: [5, 5] } },
      ],
    };
    // All three clicks via the SAME (stale, pre-undo) props object — that's
    // the mode's real stash source; a 4th click with fresh post-undo props to
    // "mimic returning to canvas" isn't needed, since syncDraft is what
    // buildEditLayer actually calls on a toolbar-driven rebuild.
    const p = props({ data: fcV1 });
    m.handleClick(click([0, 0], [0, 0]), p);
    m.handleClick(click([1, 0], [100, 0]), p);
    m.handleClick(click([1, 1], [100, 100]), p);

    // Toolbar Undo rebuilds the draft WITHOUT a canvas event.
    const fcV2: GeoJSON.FeatureCollection = { type: "FeatureCollection", features: [] };
    const onEdit2 = vi.fn();
    m.syncDraft(fcV2, onEdit2);

    m.finish();

    // Emitted through the SYNCED onEdit, not the stale one from `p`.
    expect(onEdit2).toHaveBeenCalled();
    const pTypes = (p as { onEdit: ReturnType<typeof vi.fn> }).onEdit.mock.calls.map(
      (c) => c[0].editType,
    );
    expect(pTypes).not.toContain("addFeature");
    const addFeatureCall = onEdit2.mock.calls.find((c) => c[0].editType === "addFeature");
    expect(addFeatureCall).toBeDefined();
    const updated = addFeatureCall![0].updatedData as GeoJSON.FeatureCollection;
    // Built against the synced (post-undo, empty) fcV2 + the new polygon — NOT
    // fcV1's stale point feature.
    expect(updated.features.some((f) => f.geometry.type === "Point")).toBe(false);
    expect(updated.features.some((f) => f.geometry.type === "Polygon")).toBe(true);
  });

  it("syncDraft on a never-clicked instance is a no-op (no throw, finish() still no-ops)", () => {
    const m = new KojiDrawPolygonMode();
    const fc: GeoJSON.FeatureCollection = { type: "FeatureCollection", features: [] };
    const onEdit = vi.fn();
    expect(() => m.syncDraft(fc, onEdit)).not.toThrow();
    m.finish();
    expect(onEdit).not.toHaveBeenCalled();
  });
});
