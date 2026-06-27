import { expect, test } from "vitest";
import { editModeFor } from "@/map/lib/edit-modes";
import { DrawPolygonMode, ModifyMode, ViewMode } from "@deck.gl-community/editable-layers";

test("editModeFor maps each DrawMode to its editable-layers class", () => {
  expect(editModeFor("none")).toBe(ViewMode);
  expect(editModeFor("drawPolygon")).toBe(DrawPolygonMode);
  expect(editModeFor("modify")).toBe(ModifyMode);
});
