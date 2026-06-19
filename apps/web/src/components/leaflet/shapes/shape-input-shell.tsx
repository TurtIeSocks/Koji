"use client";

import { FeatureGroup } from "react-leaflet";
import { GeomanControls } from "react-leaflet-geoman-v2";
import type { PM } from "leaflet";
import "@geoman-io/leaflet-geoman-free/dist/leaflet-geoman.css";

import { BaseMap } from "../shared-map";
import { useGeomanRHF } from "../geoman/use-geoman-rhf";
import { geojsonTypeToGeomanShape } from "../geoman/geoman-shape-mapping";
import type { BaseInputProps, GeomanShape, ShapeKind } from "../types";

interface ShapeInputShellProps extends BaseInputProps {
  shape: ShapeKind;
  multi: boolean;
  collection?: boolean;
  /**
   * Override the toolbar draw buttons. When omitted, derived from `shape`.
   * Use when a single GeoJSON output type (e.g. Polygon) can be produced by
   * multiple draw modes (Polygon, Rectangle, Circle).
   */
  geomanShapes?: GeomanShape[];
  /**
   * Optional converter that transforms the drawn `GeoJSON.Geometry` into the
   * shape stored in the form value. The second arg `prev` holds the current
   * form value at persist time, letting transforms carry forward fields like
   * `Feature.properties` across edits. Forwarded to `useGeomanRHF`.
   */
  valueTransform?: (geom: GeoJSON.Geometry, prev: unknown) => unknown;
  /**
   * Optional inverse of `valueTransform` used during hydration. Forwarded to
   * `useGeomanRHF`.
   */
  valueParse?: (stored: unknown) => GeoJSON.Geometry | null;
}

interface ShellInnerProps
  extends Pick<
    ShapeInputShellProps,
    | "source"
    | "shape"
    | "multi"
    | "collection"
    | "geomanShapes"
    | "snappable"
    | "snapDistance"
    | "pathOptions"
    | "valueTransform"
    | "valueParse"
  > {
  disabled?: boolean;
  validate?: (geom: GeoJSON.Geometry) => string | undefined;
}

function ShapeInputShell({
  source,
  shape,
  multi,
  collection,
  geomanShapes,
  zoom = 13,
  defaultCenter = [0, 0],
  height = 300,
  tileUrl,
  attribution,
  pathOptions,
  snappable = true,
  snapDistance = 20,
  label,
  helperText,
  disabled = false,
  validate,
  valueTransform,
  valueParse,
}: ShapeInputShellProps) {
  const validators = Array.isArray(validate)
    ? validate
    : validate
      ? [validate]
      : [];
  const combinedValidator = validators.length
    ? (g: GeoJSON.Geometry) => {
        for (const v of validators) {
          const err = v(g);
          if (err) return err;
        }
        return undefined;
      }
    : undefined;

  return (
    <div className="flex flex-col gap-1" data-slot="shape-input">
      {label ? <span className="text-sm font-medium">{label}</span> : null}
      <BaseMap
        zoom={zoom}
        defaultCenter={defaultCenter}
        height={height}
        tileUrl={tileUrl}
        attribution={attribution}
        testId={`${shape.toLowerCase()}-input`}
        className={
          disabled
            ? "overflow-hidden rounded-md border pointer-events-none opacity-60"
            : undefined
        }
      >
        <ShellInner
          source={source}
          shape={shape}
          multi={multi}
          collection={collection}
          geomanShapes={geomanShapes}
          snappable={snappable}
          snapDistance={snapDistance}
          pathOptions={pathOptions}
          disabled={disabled}
          validate={combinedValidator}
          valueTransform={valueTransform}
          valueParse={valueParse}
        />
      </BaseMap>
      {helperText ? (
        <div className="text-xs text-muted-foreground">{helperText}</div>
      ) : null}
    </div>
  );
}

function ShellInner({
  source,
  shape,
  multi,
  collection,
  geomanShapes,
  snappable,
  snapDistance,
  pathOptions,
  disabled,
  validate,
  valueTransform,
  valueParse,
}: ShellInnerProps) {
  const { featureGroupRef, geomanControlsProps } = useGeomanRHF({
    source,
    shape,
    multi,
    collection,
    validate,
    pathOptions,
    valueTransform,
    valueParse,
  });
  const drawShapes = geomanShapes ?? [geojsonTypeToGeomanShape(shape)];
  const toolbarOptions: PM.ToolbarOptions = {
    position: "topleft",
    drawMarker: drawShapes.includes("Marker"),
    drawCircleMarker: drawShapes.includes("CircleMarker"),
    drawPolyline: drawShapes.includes("Line"),
    drawRectangle: drawShapes.includes("Rectangle"),
    drawPolygon: drawShapes.includes("Polygon"),
    drawCircle: drawShapes.includes("Circle"),
    drawText: drawShapes.includes("Text"),
    editMode: true,
    dragMode: true,
    cutPolygon: drawShapes.some(
      (s) => s === "Polygon" || s === "Rectangle" || s === "Circle",
    ),
    removalMode: true,
    rotateMode: false,
  };
  const globalOptions: PM.GlobalOptions = {
    snappable,
    snapDistance,
    pathOptions,
  };
  if (disabled) return null;
  return (
    <FeatureGroup ref={featureGroupRef}>
      <GeomanControls
        options={toolbarOptions}
        globalOptions={globalOptions}
        {...geomanControlsProps}
      />
    </FeatureGroup>
  );
}

export { ShapeInputShell, type ShapeInputShellProps };
