"use client";

import {
  ShapeInputShell,
  type ShapeInputShellProps,
} from "./shape-input-shell";

type PointInputProps = Omit<ShapeInputShellProps, "shape" | "multi">;

function PointInput(props: PointInputProps) {
  return <ShapeInputShell {...props} shape="Point" multi={false} />;
}

export { PointInput, type PointInputProps };
