"use client";

import {
  ShapeInputShell,
  type ShapeInputShellProps,
} from "./shape-input-shell";

type MultiPointInputProps = Omit<ShapeInputShellProps, "shape" | "multi">;

function MultiPointInput(props: MultiPointInputProps) {
  return <ShapeInputShell {...props} shape="MultiPoint" multi={true} />;
}

export { MultiPointInput, type MultiPointInputProps };
