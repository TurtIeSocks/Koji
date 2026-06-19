"use client";

import { useEffect, useMemo, useRef, useState } from "react";
import { useFormContext, useWatch } from "react-hook-form";
import { GeoJSON } from "react-leaflet";
import L from "leaflet";
import simplify from "@turf/simplify";

import { BaseMap, FitBoundsOnMount } from "./shared-map";
import { MarkerIcon } from "./shared";
import { Slider } from "@/components/ui/slider";
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group";
import type { BaseInputProps } from "./types";

type SimplifyQuality = "Default" | "High";

interface SimplifyInputProps extends BaseInputProps {
  /** Initial slider value. Defaults to `0.01`. */
  tolerance?: number;
  /** Minimum slider value. Defaults to `0`. */
  minTolerance?: number;
  /** Maximum slider value. Defaults to `0.05`. */
  maxTolerance?: number;
  /** Slider step. Defaults to `0.001`. */
  step?: number;
  /** "Default" → `highQuality=false`, "High" → `highQuality=true`. */
  quality?: SimplifyQuality;
}

/**
 * Runtime guard for the input value: GeoJSON-ish (Geometry, Feature, or
 * FeatureCollection). Intentionally minimal — see component comment.
 */
const isGeoJsonLike = (v: unknown): boolean =>
  v != null &&
  typeof v === "object" &&
  "type" in v &&
  typeof (v as { type: unknown }).type === "string";

function SimplifyInput({
  source,
  zoom = 13,
  defaultCenter = [0, 0],
  height = 400,
  tileUrl,
  attribution,
  pathOptions = { color: "#3388ff" },
  tolerance: initialTolerance = 0.01,
  minTolerance = 0,
  maxTolerance = 0.05,
  step = 0.001,
  quality: initialQuality = "Default",
  label,
  helperText,
}: SimplifyInputProps) {
  const form = useFormContext();
  const value = useWatch({ name: source }) as
    | GeoJSON.Geometry
    | GeoJSON.Feature
    | GeoJSON.FeatureCollection
    | null
    | undefined;

  // Validate once on mount. Throw — the contract here is hard: callers must
  // pass GeoJSON or null. Anything else is a programmer error.
  const validatedOnceRef = useRef(false);
  if (!validatedOnceRef.current) {
    if (value != null && !isGeoJsonLike(value)) {
      throw new Error("SimplifyInput: source must be GeoJSON or null");
    }
    validatedOnceRef.current = true;
  }

  // Capture the FIRST non-null value as the "original". All slider/quality
  // changes simplify from this snapshot — sliding tolerance back to 0
  // restores the (deep-cloned) original. Stored in state (not just a ref) so
  // the simplify effect re-fires once it lands. RHF's `useWatch` may return
  // undefined on the first render and the seeded form value only on the
  // second; we must wait for that second render before snapshotting.
  const [original, setOriginal] = useState<
    GeoJSON.Geometry | GeoJSON.Feature | GeoJSON.FeatureCollection | null
  >(null);
  useEffect(() => {
    if (original == null && value != null) {
      // structuredClone so we own the data turf will mutate (mutate=true).
      setOriginal(structuredClone(value));
    }
  }, [value, original]);

  const [tolerance, setTolerance] = useState(initialTolerance);
  const [quality, setQuality] = useState<SimplifyQuality>(initialQuality);

  // Run simplify whenever tolerance/quality/original change. Until `original`
  // is captured, leave the form value alone.
  const lastWrittenRef = useRef<unknown>(undefined);
  useEffect(() => {
    if (original == null) return;
    const clone = structuredClone(original);
    const result = simplify(clone, {
      tolerance,
      highQuality: quality === "High",
      mutate: true,
    });
    // Skip the write if nothing changed (e.g. tolerance=0 first run after
    // capturing the original) — avoids dirtying the form needlessly.
    const next = JSON.stringify(result);
    if (next === JSON.stringify(lastWrittenRef.current)) return;
    lastWrittenRef.current = result;
    form.setValue(source, result, { shouldDirty: true });
  }, [original, tolerance, quality, form, source]);

  // Use the live form value for rendering — it reflects the simplified result.
  const previewGeom = value ?? null;
  const bounds = useMemo(() => {
    if (!previewGeom) return null;
    const layer = L.geoJSON(previewGeom as GeoJSON.GeoJsonObject);
    const b = layer.getBounds();
    return b.isValid() ? b : null;
  }, [previewGeom]);

  return (
    <div className="flex flex-col gap-2" data-slot="simplify-input">
      {label ? <span className="text-sm font-medium">{label}</span> : null}
      {previewGeom == null ? (
        <div
          style={{ height, width: "100%" }}
          className="flex items-center justify-center rounded-md border bg-muted/30 text-sm text-muted-foreground"
          data-slot="shape-field-empty"
        >
          No geometry available
        </div>
      ) : (
        <BaseMap
          zoom={zoom}
          defaultCenter={defaultCenter}
          height={height}
          tileUrl={tileUrl}
          attribution={attribution}
          testId="simplify-input"
        >
          <GeoJSON
            // Key forces a re-mount on every geom change so the layer reflects
            // the latest simplification — <GeoJSON> ignores subsequent `data`
            // prop changes once mounted.
            key={JSON.stringify(previewGeom)}
            data={previewGeom as GeoJSON.GeoJsonObject}
            style={() => pathOptions}
            pointToLayer={(_f, latlng) =>
              L.marker(latlng, { icon: MarkerIcon })
            }
          />
          <FitBoundsOnMount bounds={bounds} />
        </BaseMap>
      )}
      <div className="flex items-center gap-4">
        <div className="flex-1">
          <div className="mb-1 text-xs text-muted-foreground">
            Tolerance: {tolerance.toFixed(3)}
          </div>
          <Slider
            min={minTolerance}
            max={maxTolerance}
            step={step}
            value={[tolerance]}
            onValueChange={(v) => setTolerance(v[0] ?? minTolerance)}
            data-testid="simplify-tolerance-slider"
          />
        </div>
        <ToggleGroup
          type="single"
          value={quality}
          onValueChange={(v) => {
            if (v === "Default" || v === "High") setQuality(v);
          }}
          variant="outline"
          size="sm"
          data-testid="simplify-quality-toggle"
        >
          <ToggleGroupItem value="Default">Default</ToggleGroupItem>
          <ToggleGroupItem value="High">High</ToggleGroupItem>
        </ToggleGroup>
      </div>
      {helperText ? (
        <div className="text-xs text-muted-foreground">{helperText}</div>
      ) : null}
    </div>
  );
}

export { SimplifyInput, type SimplifyInputProps, type SimplifyQuality };
