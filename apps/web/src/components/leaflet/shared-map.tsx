"use client";

import { type ReactNode, useEffect } from "react";
import { MapContainer, TileLayer, useMap } from "react-leaflet";
import type * as L from "leaflet";

import { DEFAULT_ATTRIBUTION, DEFAULT_TILE_URL } from "./shared";
import type { BaseMapProps } from "./types";

interface BaseMapWrapperProps extends BaseMapProps {
  children?: ReactNode;
  testId?: string;
  className?: string;
}

const MAP_STYLE: React.CSSProperties = { height: "100%", width: "100%" };

function BaseMap({
  zoom = 13,
  defaultCenter = [0, 0],
  height = 300,
  tileUrl = DEFAULT_TILE_URL,
  attribution = DEFAULT_ATTRIBUTION,
  children,
  testId,
  className,
}: BaseMapWrapperProps) {
  return (
    <div
      style={{ height }}
      className={className ?? "overflow-hidden rounded-md border w-full"}
      data-testid={testId}
    >
      <MapContainer center={defaultCenter} zoom={zoom} style={MAP_STYLE}>
        <TileLayer url={tileUrl} attribution={attribution} />
        {children}
      </MapContainer>
    </div>
  );
}

interface FitBoundsOnMountProps {
  bounds: L.LatLngBoundsExpression | null;
  padding?: L.PointTuple;
  maxZoom?: number;
}

function FitBoundsOnMount({
  bounds,
  padding = [20, 20],
  maxZoom = 18,
}: FitBoundsOnMountProps) {
  const map = useMap();
  useEffect(() => {
    if (bounds) map.fitBounds(bounds, { padding, maxZoom });
  }, [bounds, map, padding, maxZoom]);
  return null;
}

export {
  BaseMap,
  type BaseMapWrapperProps,
  FitBoundsOnMount,
  type FitBoundsOnMountProps,
};
