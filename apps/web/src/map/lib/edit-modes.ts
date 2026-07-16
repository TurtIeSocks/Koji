import {
  ViewMode,
  DrawPolygonMode,
  DrawRectangleMode,
  DrawCircleFromCenterMode,
  ModifyMode,
  TransformMode,
  SplitPolygonMode,
} from "@deck.gl-community/editable-layers";
import { KojiDrawPolygonMode } from "./koji-draw-polygon-mode";

export type DrawMode =
  | "none"
  | "drawPolygon"
  | "drawRectangle"
  | "drawCircle"
  | "modify"
  | "transform"
  | "split"
  | "cutHole";

export interface ModeSpec {
  /** editable-layers mode constructor (typed loosely — the lib's types don't line
   *  up cleanly with @deck.gl/core's Layer; the cast lives in buildEditLayer). */
  ModeClass: unknown;
  /** Passed to EditableGeoJsonLayer.modeConfig — e.g. the boolean op for hole-cut. */
  modeConfig?: Record<string, unknown>;
  /** Clicking a feature selects it as the edit target (modify/transform). Draw
   *  modes leave this off — their clicks place vertices, not selections. */
  clickToSelect: boolean;
  /** Mode operates on an existing feature. Combined with !clickToSelect it means
   *  the shape must ALREADY be selected before entering the mode (split/cutHole). */
  needsSelection: boolean;
}

const SPECS: Record<DrawMode, ModeSpec> = {
  none: { ModeClass: ViewMode, clickToSelect: false, needsSelection: false },
  drawPolygon: { ModeClass: KojiDrawPolygonMode, clickToSelect: false, needsSelection: false },
  drawRectangle: { ModeClass: DrawRectangleMode, clickToSelect: false, needsSelection: false },
  drawCircle: { ModeClass: DrawCircleFromCenterMode, clickToSelect: false, needsSelection: false },
  modify: { ModeClass: ModifyMode, clickToSelect: true, needsSelection: true },
  // ponytail: tester reports "Move makes my fence disappear" (2026-07-15) —
  // unreproduced; suspected mechanism is TranslateMode's unclamped geodesic
  // drag + DeckMap's fit-once camera (deck-map.tsx) flinging the shape
  // off-screen. Left in deliberately. Upgrade path if a repro lands:
  // selection-gate the button + re-fit the camera after a transform drag.
  transform: { ModeClass: TransformMode, clickToSelect: true, needsSelection: true },
  split: { ModeClass: SplitPolygonMode, clickToSelect: false, needsSelection: true },
  cutHole: {
    // KEEPS the stock DrawPolygonMode — the hole preview needs the polygon
    // fill (KojiDrawPolygonMode.createTentativeFeature defers to it anyway
    // when isDrawingHole, but there's no reason to route through it here).
    ModeClass: DrawPolygonMode,
    modeConfig: { booleanOperation: "difference" },
    clickToSelect: false,
    needsSelection: true,
  },
};

export function modeSpecFor(mode: DrawMode): ModeSpec {
  return SPECS[mode];
}

/** A mode you can only use once a shape is already selected (draw-a-hole /
 *  draw-a-split-line), as opposed to modify/transform which select on click. */
export function requiresPriorSelection(mode: DrawMode): boolean {
  const s = SPECS[mode];
  return s.needsSelection && !s.clickToSelect;
}

/** A fresh mode instance for the layer. Passing an INSTANCE (not the class)
 *  lets the toolbar hold a handle to it (Done/Cancel call finish()/cancel());
 *  the layer only re-instantiates when the mode prop's identity changes, so
 *  the instance — and its in-progress click sequence — survives layer
 *  rebuilds. Memoize per mode switch (deck-geojson-input.tsx). */
export function createModeInstance(mode: DrawMode): unknown {
  const Cls = SPECS[mode].ModeClass as new () => unknown;
  return new Cls();
}
