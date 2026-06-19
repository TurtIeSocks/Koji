"use client";

import {
  ShapeInputShell,
  type ShapeInputShellProps,
} from "./shape-input-shell";

type MultiLineStringInputProps = Omit<ShapeInputShellProps, "shape" | "multi">;

function MultiLineStringInput(props: MultiLineStringInputProps) {
  return <ShapeInputShell {...props} shape="MultiLineString" multi={true} />;
}

export { MultiLineStringInput, type MultiLineStringInputProps };
