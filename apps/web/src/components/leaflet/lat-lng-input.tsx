"use client";

import { type ReactNode, useEffect, useRef } from "react";
import L from "leaflet";
import {
  MapContainer,
  TileLayer,
  Marker,
  useMap,
  useMapEvents,
} from "react-leaflet";
import { useFormContext } from "react-hook-form";

import { DEFAULT_ATTRIBUTION, DEFAULT_TILE_URL, MarkerIcon } from "./shared";

interface LatLngInputProps {
  latSource: string;
  lngSource: string;
  defaultPosition?: [number, number];
  zoom?: number;
  height?: number | string;
  tileUrl?: string;
  attribution?: string;
  label?: ReactNode;
  helperText?: ReactNode;
}

interface DraggableMarkerProps {
  position: [number, number];
  onChange: (lat: number, lng: number) => void;
}

function DraggableMarker({ position, onChange }: DraggableMarkerProps) {
  const markerRef = useRef<L.Marker>(null);
  useMapEvents({
    click: (e) => onChange(e.latlng.lat, e.latlng.lng),
  });
  return (
    <Marker
      position={position}
      icon={MarkerIcon}
      draggable
      ref={markerRef}
      eventHandlers={{
        dragend: () => {
          const ll = markerRef.current?.getLatLng();
          if (ll) onChange(ll.lat, ll.lng);
        },
      }}
    />
  );
}

interface RecenterOnChangeProps {
  position: [number, number];
}

function RecenterOnChange({ position }: RecenterOnChangeProps) {
  const map = useMap();
  // biome-ignore lint/correctness/useExhaustiveDependencies: map from useMap() is stable; recenter only when the lat/lng coords change, not on map identity
  useEffect(() => {
    map.setView(position, map.getZoom(), { animate: true });
  }, [position[0], position[1]]);
  return null;
}

/**
 * Lat/lng map form input. Renders a draggable Leaflet marker; clicking the map or dragging the marker
 * updates two React Hook Form fields (`latSource` and `lngSource`) via `setValue`.
 *
 * Must be rendered inside a React Hook Form context (e.g. `<SimpleForm>`, `<Form>`).
 *
 * @example
 * <Create>
 *   <SimpleForm>
 *     <LatLngInput
 *       latSource="lat"
 *       lngSource="lng"
 *       defaultPosition={[48.85, 2.35]}
 *       height={300}
 *       label="Location"
 *       helperText="Click or drag the marker to set a location."
 *     />
 *   </SimpleForm>
 * </Create>
 */
function LatLngInput({
  latSource,
  lngSource,
  defaultPosition = [0, 0],
  zoom = 13,
  height = 300,
  tileUrl = DEFAULT_TILE_URL,
  attribution = DEFAULT_ATTRIBUTION,
  label,
  helperText,
}: LatLngInputProps) {
  const form = useFormContext();
  const latValue = form?.watch(latSource) as number | undefined;
  const lngValue = form?.watch(lngSource) as number | undefined;
  const lat = typeof latValue === "number" ? latValue : defaultPosition[0];
  const lng = typeof lngValue === "number" ? lngValue : defaultPosition[1];

  const handleChange = (newLat: number, newLng: number) => {
    form?.setValue(latSource, newLat, { shouldDirty: true });
    form?.setValue(lngSource, newLng, { shouldDirty: true });
  };

  return (
    <div
      className="flex flex-col gap-1"
      data-slot="lat-lng-input"
      data-testid="lat-lng-input"
    >
      {label ? <span className="text-sm font-medium">{label}</span> : null}
      <div
        style={{ height, width: "100%" }}
        className="overflow-hidden rounded-md border"
      >
        <MapContainer
          center={[lat, lng]}
          zoom={zoom}
          style={{ height: "100%", width: "100%" }}
        >
          <TileLayer url={tileUrl} attribution={attribution} />
          <DraggableMarker position={[lat, lng]} onChange={handleChange} />
          <RecenterOnChange position={[lat, lng]} />
        </MapContainer>
      </div>
      {helperText ? (
        <div className="text-xs text-muted-foreground">{helperText}</div>
      ) : null}
    </div>
  );
}

export { LatLngInput, type LatLngInputProps };
