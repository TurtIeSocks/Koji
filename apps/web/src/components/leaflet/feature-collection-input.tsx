"use client";

import type * as L from "leaflet";
import type { PM } from "leaflet";
import { FeatureGroup } from "react-leaflet";
import { GeomanControls } from "react-leaflet-geoman-v2";
import "@geoman-io/leaflet-geoman-free/dist/leaflet-geoman.css";

import { BaseMap } from "./shared-map";
import { useGeomanRHF } from "./geoman/use-geoman-rhf";
import type { BaseInputProps, GeomanShape, ShapeKind } from "./types";

/**
 * GeoJSON shape kinds expressible as a `Feature` in a `FeatureCollection`.
 * `GeometryCollection` is excluded — a FeatureCollection of GeometryCollections
 * is legal GeoJSON but degenerate and harder to draw.
 */
type DrawableShape = Exclude<ShapeKind, "GeometryCollection">;

const SHAPE_TO_GEOMAN: Record<DrawableShape, GeomanShape> = {
  Point: "Marker",
  MultiPoint: "Marker",
  LineString: "Line",
  MultiLineString: "Line",
  Polygon: "Polygon",
  MultiPolygon: "Polygon",
};

const DEFAULT_SHAPES: DrawableShape[] = ["Point", "LineString", "Polygon"];

interface FeatureCollectionInputProps extends BaseInputProps {
  /** Which draw tools to expose. Defaults to `["Point", "LineString", "Polygon"]`. */
  allowedShapes?: DrawableShape[];
}

/**
 * Form input for drawing/editing a `GeoJSON.FeatureCollection`. Each drawn
 * shape becomes a `Feature` in the collection; `properties` are preserved
 * by index across edits (best-effort — order may shift on removal).
 */
function FeatureCollectionInput({
  source,
  allowedShapes = DEFAULT_SHAPES,
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
}: FeatureCollectionInputProps) {
  const geomanShapes = Array.from(
    new Set(allowedShapes.map((s) => SHAPE_TO_GEOMAN[s])),
  );
  return (
    <div className="flex flex-col gap-1" data-slot="feature-collection-input">
      {label ? <span className="text-sm font-medium">{label}</span> : null}
      <BaseMap
        zoom={zoom}
        defaultCenter={defaultCenter}
        height={height}
        tileUrl={tileUrl}
        attribution={attribution}
        testId="feature-collection-input"
      >
        <Inner
          source={source}
          shapes={geomanShapes}
          snappable={snappable}
          snapDistance={snapDistance}
          pathOptions={pathOptions}
        />
      </BaseMap>
      {helperText ? (
        <div className="text-xs text-muted-foreground">{helperText}</div>
      ) : null}
    </div>
  );
}

interface InnerProps {
  source: string;
  shapes: GeomanShape[];
  snappable: boolean;
  snapDistance: number;
  pathOptions?: L.PathOptions;
}

function Inner({
  source,
  shapes,
  snappable,
  snapDistance,
  pathOptions,
}: InnerProps) {
  // Shape kind doesn't matter for FC mode — `featureCollection: true` overrides
  // multi/collection branches in the hook.
  const { featureGroupRef, geomanControlsProps } = useGeomanRHF({
    source,
    shape: "Point",
    multi: false,
    featureCollection: true,
    pathOptions,
  });
  const toolbarOptions: PM.ToolbarOptions = {
    position: "topleft",
    drawMarker: shapes.includes("Marker"),
    drawCircleMarker: shapes.includes("CircleMarker"),
    drawPolyline: shapes.includes("Line"),
    drawRectangle: shapes.includes("Rectangle"),
    drawPolygon: shapes.includes("Polygon"),
    drawCircle: shapes.includes("Circle"),
    drawText: shapes.includes("Text"),
    editMode: true,
    dragMode: true,
    cutPolygon: shapes.includes("Polygon"),
    removalMode: true,
    rotateMode: false,
  };
  const globalOptions: PM.GlobalOptions = {
    snappable,
    snapDistance,
    pathOptions,
  };
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

export { FeatureCollectionInput, type FeatureCollectionInputProps };
