"use client";
import { useEffect } from "react";
import { useFormContext, useWatch } from "react-hook-form";

import {
  ShapeInputShell,
  type ShapeInputShellProps,
} from "./shape-input-shell";
import { snapToRoadsOnce } from "../osm/use-osm-snap-to-roads";

interface LineStringInputProps
  extends Omit<ShapeInputShellProps, "shape" | "multi"> {
  snapToRoads?: boolean;
}

function LineStringInput({
  snapToRoads,
  source,
  ...rest
}: LineStringInputProps) {
  const form = useFormContext();
  const value = useWatch({ name: source }) as
    | GeoJSON.LineString
    | null
    | undefined;

  useEffect(() => {
    if (!snapToRoads || !value || value.coordinates.length < 2) return;
    let cancelled = false;
    snapToRoadsOnce(value).then((snapped) => {
      if (cancelled || !snapped) return;
      // Idempotency guard: if already snapped, skip the write to avoid an effect loop.
      if (
        JSON.stringify(snapped.coordinates) ===
        JSON.stringify(value.coordinates)
      ) {
        return;
      }
      form.setValue(source, snapped, { shouldDirty: true });
    });
    return () => {
      cancelled = true;
    };
  }, [snapToRoads, source, value, form]);

  return (
    <ShapeInputShell
      {...rest}
      source={source}
      shape="LineString"
      multi={false}
    />
  );
}

export { LineStringInput, type LineStringInputProps };
