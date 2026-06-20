"use client";

import { type ReactNode, useEffect } from "react";
import { MapContainer, TileLayer, useMap } from "react-leaflet";
import type * as L from "leaflet";

import { DEFAULT_ATTRIBUTION, DEFAULT_TILE_URL } from "./shared";
import { useStartCenter } from "./use-start-center";
import type { BaseMapProps } from "./types";

interface BaseMapWrapperProps extends BaseMapProps {
  children?: ReactNode;
  testId?: string;
  className?: string;
}

const MAP_STYLE: React.CSSProperties = { height: "100%", width: "100%" };

function BaseMap({
  zoom = 13,
  defaultCenter,
  height = 300,
  tileUrl = DEFAULT_TILE_URL,
  attribution = DEFAULT_ATTRIBUTION,
  children,
  testId,
  className,
}: BaseMapWrapperProps) {
  // Empty / pre-fit center: explicit prop wins, else the server's START_LAT/LON.
  const startCenter = useStartCenter();
  const center = defaultCenter ?? startCenter;
  return (
    <div
      style={{ height }}
      className={className ?? "overflow-hidden rounded-md border w-full"}
      data-testid={testId}
    >
      <MapContainer center={center} zoom={zoom} style={MAP_STYLE}>
        <TileLayer url={tileUrl} attribution={attribution} />
        <RecenterFromZero center={center} />
        {children}
      </MapContainer>
    </div>
  );
}

/**
 * Re-applies the start center once the async config resolves — but ONLY while
 * the map still sits at [0,0]. A map that already moved (a child `fitBounds` to
 * a geometry, or a user pan) is left alone, so this never fights geometry fit.
 */
function RecenterFromZero({ center }: { center: L.LatLngExpression }) {
  const map = useMap();
  useEffect(() => {
    const c = map.getCenter();
    if (Math.abs(c.lat) < 1e-6 && Math.abs(c.lng) < 1e-6) {
      map.setView(center);
    }
  }, [center, map]);
  return null;
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
