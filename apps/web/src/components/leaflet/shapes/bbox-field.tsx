"use client";

import { useMemo } from "react";
import { RecordContextProvider, useRecordContext } from "shadmin-core";

import {
  ShapeFieldShell,
  type ShapeFieldShellProps,
} from "./shape-field-shell";

type BBoxFieldProps = ShapeFieldShellProps;

function BBoxField({ source, ...rest }: BBoxFieldProps) {
  const record = useRecordContext();
  const bbox = record?.[source] as GeoJSON.BBox | null | undefined;

  const recordWithPolygon = useMemo(() => {
    if (!bbox) return record;
    const [w, s, e, n] = bbox;
    const polygon: GeoJSON.Polygon = {
      type: "Polygon",
      coordinates: [
        [
          [w, s],
          [e, s],
          [e, n],
          [w, n],
          [w, s],
        ],
      ],
    };
    return { ...record, [source]: polygon };
  }, [record, bbox, source]);

  if (!bbox) {
    // Pass through to shell so it renders its own empty state.
    return <ShapeFieldShell source={source} {...rest} />;
  }

  // Wrap with a record that exposes the polygon at `source` so the shell renders it.
  return (
    <RecordContextProvider value={recordWithPolygon}>
      <ShapeFieldShell source={source} {...rest} />
    </RecordContextProvider>
  );
}

export { BBoxField, type BBoxFieldProps };
