"use client";

import { useState } from "react";
import { useFormContext } from "react-hook-form";

import {
  Command,
  CommandEmpty,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
} from "@/components/ui/command";

import { useGeocode, type UseGeocodeOptions } from "./use-geocode";

interface GeocodingInputProps extends UseGeocodeOptions {
  source: string;
  latSource?: string;
  lngSource?: string;
  bboxSource?: string;
  placeholder?: string;
  label?: string;
}

/**
 * Inline-input combobox for address search. The `<CommandInput>` lives inside a
 * single, stable `<Command>` subtree at all times — the results dropdown is a
 * conditionally rendered sibling, not a portalled `Popover` whose trigger
 * remounts the input. That keeps focus on the input across keystrokes while the
 * `useGeocode` query results update.
 */
const GeocodingInput = ({
  source,
  latSource,
  lngSource,
  bboxSource,
  placeholder = "Search address or place…",
  label,
  ...geocodeOpts
}: GeocodingInputProps) => {
  const form = useFormContext();
  const [query, setQuery] = useState("");
  const [open, setOpen] = useState(false);
  const minChars = geocodeOpts.minChars ?? 3;
  const { data: results = [], isFetching } = useGeocode(query, geocodeOpts);

  const onSelect = (r: (typeof results)[number]) => {
    form.setValue(source, r.displayName, { shouldDirty: true });
    if (latSource) form.setValue(latSource, r.lat, { shouldDirty: true });
    if (lngSource) form.setValue(lngSource, r.lng, { shouldDirty: true });
    if (bboxSource && r.bbox)
      form.setValue(bboxSource, r.bbox, { shouldDirty: true });
    setQuery(r.displayName);
    setOpen(false);
  };

  const showList = open && query.length >= minChars;

  return (
    <div className="flex flex-col gap-1" data-slot="geocoding-input">
      {label ? <span className="text-sm font-medium">{label}</span> : null}
      <Command shouldFilter={false} className="rounded-md border">
        <CommandInput
          placeholder={placeholder}
          value={query}
          onValueChange={(v) => {
            setQuery(v);
            setOpen(v.length >= minChars);
          }}
          onFocus={() => {
            if (query.length >= minChars) setOpen(true);
          }}
          onBlur={() => {
            // Defer so a CommandItem onSelect click resolves before close.
            setTimeout(() => setOpen(false), 150);
          }}
        />
        {showList ? (
          <CommandList>
            {isFetching ? (
              <CommandEmpty>Loading…</CommandEmpty>
            ) : results.length === 0 ? (
              <CommandEmpty>No results</CommandEmpty>
            ) : (
              <CommandGroup>
                {results.map((r, i) => (
                  <CommandItem
                    key={r.displayName ?? i}
                    onSelect={() => onSelect(r)}
                  >
                    <div className="flex flex-col">
                      <span>{r.displayName}</span>
                      {r.type ? (
                        <span className="text-xs text-muted-foreground">
                          {r.type}
                        </span>
                      ) : null}
                    </div>
                  </CommandItem>
                ))}
              </CommandGroup>
            )}
          </CommandList>
        ) : null}
      </Command>
    </div>
  );
};

export { type GeocodingInputProps, GeocodingInput };
