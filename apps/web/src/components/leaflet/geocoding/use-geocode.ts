"use client";

import { useEffect, useState } from "react";
import { useQuery } from "@tanstack/react-query";

import {
  nominatimProvider,
  type GeocodingProvider,
  type SearchOptions,
  type GeocodeResult,
} from "./nominatim-client";

interface UseGeocodeOptions extends SearchOptions {
  provider?: GeocodingProvider;
  minChars?: number;
  debounceMs?: number;
  enabled?: boolean;
}

const useGeocode = (query: string, opts: UseGeocodeOptions = {}) => {
  const provider = opts.provider ?? nominatimProvider;
  const minChars = opts.minChars ?? 3;
  const debounceMs = opts.debounceMs ?? 300;
  const [debounced, setDebounced] = useState(query);
  useEffect(() => {
    const t = setTimeout(() => setDebounced(query), debounceMs);
    return () => clearTimeout(t);
  }, [query, debounceMs]);

  return useQuery<GeocodeResult[]>({
    queryKey: [
      "geocode",
      debounced,
      opts.countryCodes?.join(",") ?? null,
      opts.viewBox?.join(",") ?? null,
    ],
    queryFn: () => provider.search(debounced, opts),
    enabled: (opts.enabled ?? true) && debounced.length >= minChars,
    staleTime: 5 * 60 * 1000,
  });
};

export { type UseGeocodeOptions, useGeocode };
