"use client";

import { MapContainer, TileLayer, Marker } from "react-leaflet";
import { useRecordContext } from "shadmin-core";

import { DEFAULT_ATTRIBUTION, DEFAULT_TILE_URL, MarkerIcon } from "./shared";

interface LatLngFieldProps {
  latSource: string;
  lngSource: string;
  zoom?: number;
  height?: number | string;
  tileUrl?: string;
  attribution?: string;
}

/**
 * Displays a Leaflet map with a marker at the lat/lng position read from the current record.
 *
 * Reads two numeric fields from the record context (`latSource` and `lngSource`) and renders
 * an interactive (non-editable) tile map. Returns an empty-state panel when either coordinate is missing.
 *
 * @example
 * <Show>
 *   <SimpleShowLayout>
 *     <TextField source="name" />
 *     <LatLngField latSource="lat" lngSource="lng" zoom={13} height={300} />
 *   </SimpleShowLayout>
 * </Show>
 */
function LatLngField({
  latSource,
  lngSource,
  zoom = 13,
  height = 300,
  tileUrl = DEFAULT_TILE_URL,
  attribution = DEFAULT_ATTRIBUTION,
}: LatLngFieldProps) {
  const record = useRecordContext();
  const lat = record?.[latSource] as number | undefined;
  const lng = record?.[lngSource] as number | undefined;
  if (lat === undefined || lng === undefined) {
    return (
      <div
        style={{ height, width: "100%" }}
        className="flex items-center justify-center rounded-md border bg-muted/30 text-sm text-muted-foreground"
        data-slot="lat-lng-field-empty"
      >
        No coordinates available
      </div>
    );
  }
  return (
    <div
      style={{ height, width: "100%" }}
      data-slot="lat-lng-field"
      data-testid="lat-lng-field"
    >
      <MapContainer
        center={[lat, lng]}
        zoom={zoom}
        scrollWheelZoom={false}
        style={{ height: "100%", width: "100%" }}
      >
        <TileLayer url={tileUrl} attribution={attribution} />
        <Marker position={[lat, lng]} icon={MarkerIcon} />
      </MapContainer>
    </div>
  );
}

export { LatLngField, type LatLngFieldProps };
