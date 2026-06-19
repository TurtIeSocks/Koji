"use client";

import { useQuery } from "@tanstack/react-query";

import {
  nominatimProvider,
  type GeocodingProvider,
  type ReverseOptions,
  type GeocodeResult,
} from "./nominatim-client";

interface UseReverseGeocodeOptions extends ReverseOptions {
  provider?: GeocodingProvider;
  enabled?: boolean;
}

const round5 = (n: number) => Math.round(n * 1e5) / 1e5;

const useReverseGeocode = (
  coords: { lat: number | undefined; lng: number | undefined },
  opts: UseReverseGeocodeOptions = {},
) => {
  const provider = opts.provider ?? nominatimProvider;
  const enabled =
    (opts.enabled ?? true) &&
    typeof coords.lat === "number" &&
    typeof coords.lng === "number";
  return useQuery<GeocodeResult | null>({
    queryKey: [
      "reverse-geocode",
      enabled ? round5(coords.lat!) : null,
      enabled ? round5(coords.lng!) : null,
    ],
    queryFn: () => provider.reverse(coords.lat!, coords.lng!, opts),
    enabled,
    staleTime: 5 * 60 * 1000,
  });
};

export { type UseReverseGeocodeOptions, useReverseGeocode };
