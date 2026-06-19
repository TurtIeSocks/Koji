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
 * Maps GeoJSON shape kinds to the corresponding Geoman toolbar shape.
 */
const SHAPE_TO_GEOMAN: Record<ShapeKind, GeomanShape> = {
  Point: "Marker",
  MultiPoint: "Marker",
  LineString: "Line",
  MultiLineString: "Line",
  Polygon: "Polygon",
  MultiPolygon: "Polygon",
  GeometryCollection: "Marker",
};

const DEFAULT_SHAPES: ShapeKind[] = ["Point", "LineString", "Polygon"];

interface GeoJsonInputProps extends BaseInputProps {
  /**
   * Which GeoJSON shape kinds the user can draw. Each maps to a Geoman
   * draw button; duplicates (e.g. Point + MultiPoint both → Marker) are
   * deduped.
   */
  shapes?: ShapeKind[];
  /**
   * When true, persists as a `GeometryCollection` of every shape on the
   * map. When false (default), keeps only the most recently drawn shape.
   */
  collection?: boolean;
}

/**
 * Polymorphic input for drawing any GeoJSON geometry. Renders a Geoman
 * toolbar with one button per allowed shape kind. Defaults to allowing
 * Point, LineString, and Polygon.
 */
function GeoJsonInput({
  source,
  shapes = DEFAULT_SHAPES,
  collection = false,
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
}: GeoJsonInputProps) {
  const geomanShapes = Array.from(
    new Set(shapes.map((s) => SHAPE_TO_GEOMAN[s])),
  );
  return (
    <div className="flex flex-col gap-1" data-slot="geojson-input">
      {label ? <span className="text-sm font-medium">{label}</span> : null}
      <BaseMap
        zoom={zoom}
        defaultCenter={defaultCenter}
        height={height}
        tileUrl={tileUrl}
        attribution={attribution}
        testId="geojson-input"
      >
        <GeoJsonInner
          source={source}
          shapes={geomanShapes}
          collection={collection}
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

interface GeoJsonInnerProps {
  source: string;
  shapes: GeomanShape[];
  collection: boolean;
  snappable: boolean;
  snapDistance: number;
  pathOptions?: L.PathOptions;
}

function GeoJsonInner({
  source,
  shapes,
  collection,
  snappable,
  snapDistance,
  pathOptions,
}: GeoJsonInnerProps) {
  // Sentinel shape: combineMulti falls through to "keep most recent" when
  // shape isn't a Multi* type. When `collection` is true, the hook writes
  // a GeometryCollection regardless of shape.
  const { featureGroupRef, geomanControlsProps } = useGeomanRHF({
    source,
    shape: "GeometryCollection",
    multi: true,
    collection,
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

export { GeoJsonInput, type GeoJsonInputProps };
