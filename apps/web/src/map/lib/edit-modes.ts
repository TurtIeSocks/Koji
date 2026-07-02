import {
  ViewMode,
  DrawPolygonMode,
  DrawRectangleMode,
  DrawCircleFromCenterMode,
  ModifyMode,
  TransformMode,
  SplitPolygonMode,
} from "@deck.gl-community/editable-layers";

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
  drawPolygon: { ModeClass: DrawPolygonMode, clickToSelect: false, needsSelection: false },
  drawRectangle: { ModeClass: DrawRectangleMode, clickToSelect: false, needsSelection: false },
  drawCircle: { ModeClass: DrawCircleFromCenterMode, clickToSelect: false, needsSelection: false },
  modify: { ModeClass: ModifyMode, clickToSelect: true, needsSelection: true },
  transform: { ModeClass: TransformMode, clickToSelect: true, needsSelection: true },
  split: { ModeClass: SplitPolygonMode, clickToSelect: false, needsSelection: true },
  cutHole: {
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
