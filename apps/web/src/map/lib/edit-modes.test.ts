import { expect, test } from "vitest";
import { modeSpecFor } from "@/map/lib/edit-modes";
import {
  DrawPolygonMode,
  DrawRectangleMode,
  DrawCircleFromCenterMode,
  ModifyMode,
  TransformMode,
  SplitPolygonMode,
  ViewMode,
} from "@deck.gl-community/editable-layers";

test("modeSpecFor maps each DrawMode to its editable-layers class", () => {
  expect(modeSpecFor("none").ModeClass).toBe(ViewMode);
  expect(modeSpecFor("drawPolygon").ModeClass).toBe(DrawPolygonMode);
  expect(modeSpecFor("drawRectangle").ModeClass).toBe(DrawRectangleMode);
  expect(modeSpecFor("drawCircle").ModeClass).toBe(DrawCircleFromCenterMode);
  expect(modeSpecFor("modify").ModeClass).toBe(ModifyMode);
  expect(modeSpecFor("transform").ModeClass).toBe(TransformMode);
  expect(modeSpecFor("split").ModeClass).toBe(SplitPolygonMode);
});

test("cutHole draws a polygon that subtracts (difference) from the selected shape", () => {
  const spec = modeSpecFor("cutHole");
  expect(spec.ModeClass).toBe(DrawPolygonMode);
  expect(spec.modeConfig).toEqual({ booleanOperation: "difference" });
  expect(spec.needsSelection).toBe(true);
});

test("click-to-select is on only for the edit-existing modes (modify/transform)", () => {
  expect(modeSpecFor("modify").clickToSelect).toBe(true);
  expect(modeSpecFor("transform").clickToSelect).toBe(true);
  // Draw modes place vertices on click — they must NOT hijack clicks to select.
  expect(modeSpecFor("drawPolygon").clickToSelect).toBe(false);
  expect(modeSpecFor("cutHole").clickToSelect).toBe(false);
  expect(modeSpecFor("split").clickToSelect).toBe(false);
});

test("split/cutHole require a prior selection; modify/transform make their own", () => {
  // needsSelection && !clickToSelect ⇒ the shape must already be selected.
  expect(modeSpecFor("split").needsSelection && !modeSpecFor("split").clickToSelect).toBe(true);
  expect(modeSpecFor("cutHole").needsSelection && !modeSpecFor("cutHole").clickToSelect).toBe(true);
  expect(modeSpecFor("modify").needsSelection && !modeSpecFor("modify").clickToSelect).toBe(false);
});
