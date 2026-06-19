import type * as L from "leaflet";
import type { ReactNode } from "react";

type ShapeKind =
  | "Point"
  | "MultiPoint"
  | "LineString"
  | "MultiLineString"
  | "Polygon"
  | "MultiPolygon"
  | "GeometryCollection";

type GeomanShape =
  | "Marker"
  | "CircleMarker"
  | "Line"
  | "Polygon"
  | "Rectangle"
  | "Circle"
  | "Text";

interface BaseMapProps {
  zoom?: number;
  defaultCenter?: [number, number];
  height?: number | string;
  tileUrl?: string;
  attribution?: string;
}

interface BaseFieldProps extends BaseMapProps {
  source: string;
  pathOptions?: L.PathOptions;
  markerIcon?: L.Icon | L.DivIcon;
  fitBounds?: boolean;
  emptyText?: ReactNode;
}

interface BaseInputProps extends BaseMapProps {
  source: string;
  label?: ReactNode;
  helperText?: ReactNode;
  disabled?: boolean;
  pathOptions?: L.PathOptions;
  snappable?: boolean;
  snapDistance?: number;
  validate?:
    | ((v: unknown) => string | undefined)
    | Array<(v: unknown) => string | undefined>;
}

export {
  type ShapeKind,
  type GeomanShape,
  type BaseMapProps,
  type BaseFieldProps,
  type BaseInputProps,
};
