"use client";

import { useRecordContext } from "shadmin-core";

import { Skeleton } from "@/components/ui/skeleton";

import {
  useReverseGeocode,
  type UseReverseGeocodeOptions,
} from "./use-reverse-geocode";

interface ReverseGeocodeFieldProps extends UseReverseGeocodeOptions {
  latSource: string;
  lngSource: string;
  format?: "full" | "street" | "city";
  className?: string;
}

const ReverseGeocodeField = ({
  latSource,
  lngSource,
  format = "full",
  className,
  ...opts
}: ReverseGeocodeFieldProps) => {
  const record = useRecordContext();
  const lat = record?.[latSource] as number | undefined;
  const lng = record?.[lngSource] as number | undefined;
  const { data, isLoading } = useReverseGeocode({ lat, lng }, opts);

  if (lat == null || lng == null) return null;

  if (isLoading) {
    return (
      <Skeleton
        className="inline-block h-4 w-48"
        data-slot="reverse-geocode-field-loading"
        data-testid="reverse-geocode-field-loading"
      />
    );
  }

  const fallback = `${lat.toFixed(4)}, ${lng.toFixed(4)}`;
  const display = data ? data.displayName : fallback;
  const text =
    format === "full"
      ? display
      : display
          .split(",")
          .slice(0, format === "street" ? 1 : 2)
          .join(",");

  return (
    <span className={className} data-slot="reverse-geocode-field">
      {text}
    </span>
  );
};

export { type ReverseGeocodeFieldProps, ReverseGeocodeField };
