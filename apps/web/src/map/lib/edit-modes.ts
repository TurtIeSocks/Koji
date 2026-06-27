import {
  ViewMode,
  DrawPolygonMode,
  DrawRectangleMode,
  ModifyMode,
  TranslateMode,
} from "@deck.gl-community/editable-layers";

export type DrawMode = "none" | "drawPolygon" | "drawRectangle" | "modify" | "translate";

const MAP = {
  none: ViewMode,
  drawPolygon: DrawPolygonMode,
  drawRectangle: DrawRectangleMode,
  modify: ModifyMode,
  translate: TranslateMode,
} as const;

export function editModeFor(mode: DrawMode): unknown {
  return MAP[mode];
}
