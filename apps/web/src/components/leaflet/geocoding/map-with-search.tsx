"use client";

import { useEffect } from "react";
import { useFormContext, useWatch } from "react-hook-form";

import { LatLngInput } from "../lat-lng-input";
import { GeocodingInput } from "./geocoding-input";
import { useReverseGeocode } from "./use-reverse-geocode";

interface MapWithSearchProps {
  latSource: string;
  lngSource: string;
  addressSource: string;
  height?: number | string;
  defaultZoom?: number;
}

/**
 * Composite map input that combines a draggable Leaflet marker (`<LatLngInput>`)
 * with an address search combobox (`<GeocodingInput>`). Two-way sync:
 * - Selecting a search result updates lat/lng/address; the marker recenters.
 * - Dragging the marker triggers a reverse geocode that updates the address field.
 *
 * Must be rendered inside a React Hook Form context (e.g. `<SimpleForm>`).
 */
const MapWithSearch = ({
  latSource,
  lngSource,
  addressSource,
  height = 400,
  defaultZoom = 13,
}: MapWithSearchProps) => {
  const form = useFormContext();
  const lat = useWatch({ name: latSource }) as number | undefined;
  const lng = useWatch({ name: lngSource }) as number | undefined;
  const reverse = useReverseGeocode({ lat, lng });

  // When marker is dragged -> coords change -> reverse geocode -> update address.
  useEffect(() => {
    if (reverse.data?.displayName) {
      form.setValue(addressSource, reverse.data.displayName, {
        shouldDirty: true,
      });
    }
  }, [reverse.data, addressSource, form]);

  return (
    <div className="flex flex-col gap-2" data-slot="map-with-search">
      <GeocodingInput
        source={addressSource}
        latSource={latSource}
        lngSource={lngSource}
      />
      <LatLngInput
        latSource={latSource}
        lngSource={lngSource}
        defaultPosition={lat != null && lng != null ? [lat, lng] : [0, 0]}
        zoom={defaultZoom}
        height={height}
      />
    </div>
  );
};

export { type MapWithSearchProps, MapWithSearch };
