"use client";

import { useCallback, useEffect, useRef } from "react";
import { useFormContext, useWatch } from "react-hook-form";
import type L from "leaflet";
import type { PM } from "leaflet";

import type { ShapeKind } from "../types";
import { geometryToLayer, layerToGeometry } from "./geoman-shape-mapping";

interface UseGeomanRHFOptions {
  source: string;
  shape: ShapeKind;
  multi: boolean;
  collection?: boolean;
  /**
   * When true, persist as a `GeoJSON.FeatureCollection` of every drawn layer,
   * preserving per-feature `properties` from the previous form value by index.
   * Overrides `collection` / `multi` for the persistence and hydration branches.
   */
  featureCollection?: boolean;
  validate?: (geom: GeoJSON.Geometry) => string | undefined;
  pathOptions?: L.PathOptions;
  markerIcon?: L.Icon | L.DivIcon;
  /**
   * Optional converter that transforms the drawn `GeoJSON.Geometry` into the
   * shape stored in the form value. Used by `BBoxInput` to persist
   * `[w, s, e, n]` instead of a `Polygon`, and by `FeatureInput` to wrap the
   * geometry in a `GeoJSON.Feature` while preserving its `properties`. The
   * `prev` arg holds the current form value (read via `form.getValues`),
   * letting transforms preserve fields like `Feature.properties`.
   */
  valueTransform?: (geom: GeoJSON.Geometry, prev: unknown) => unknown;
  /**
   * Optional inverse of `valueTransform`: parses the stored value back into a
   * `GeoJSON.Geometry` for hydration. Return `null` for malformed input.
   * Defaults to an identity-with-type-check.
   */
  valueParse?: (stored: unknown) => GeoJSON.Geometry | null;
}

interface UseGeomanRHFReturn {
  featureGroupRef: React.MutableRefObject<L.FeatureGroup | null>;
  geomanControlsProps: {
    onCreate: PM.CreateEventHandler;
    onUpdate: PM.UpdateEventHandler;
    onLayerRemove: PM.RemoveEventHandler;
    onMapCut: PM.CutEventHandler;
    onLayerCut: PM.CutEventHandler;
  };
}

const useGeomanRHF = ({
  source,
  shape,
  multi,
  collection,
  featureCollection,
  validate,
  pathOptions,
  markerIcon,
  valueTransform,
  valueParse,
}: UseGeomanRHFOptions): UseGeomanRHFReturn => {
  const form = useFormContext();
  const featureGroupRef = useRef<L.FeatureGroup | null>(null);
  const value = useWatch({ name: source }) as unknown;
  const hydrated = useRef(false);
  // Tracks the most recent value this hook wrote into the form. Used to dedup
  // the echo `useWatch` fires after our own `setValue`, so the hydration effect
  // only rebuilds the layer when the form value changes from outside the hook.
  const lastWrittenValue = useRef<unknown>(undefined);

  // Hydration / re-hydration effect: runs when the form value differs from
  // what we last wrote. The featureGroupRef is populated by the caller via
  // <FeatureGroup ref={...}>; this effect polls for it on first mount and
  // rebuilds layers on every external change to `value` thereafter.
  useEffect(() => {
    const group = featureGroupRef.current;
    if (!group) return;

    const addFCLayers = () => {
      const fc = value as GeoJSON.FeatureCollection | null | undefined;
      if (fc?.type !== "FeatureCollection") return;
      fc.features?.forEach((feat) => {
        if (!feat?.geometry) return;
        const layer = geometryToLayer(feat.geometry, pathOptions, markerIcon);
        group.addLayer(layer);
      });
    };

    const buildLayer = () => {
      if (value == null) return null;
      const geom = valueParse
        ? valueParse(value)
        : ((value as GeoJSON.Geometry | null | undefined) ?? null);
      if (!geom) return null;
      return geometryToLayer(geom, pathOptions, markerIcon);
    };

    if (!hydrated.current) {
      if (featureCollection) {
        addFCLayers();
      } else {
        const layer = buildLayer();
        if (layer) group.addLayer(layer);
      }
      lastWrittenValue.current = value;
      hydrated.current = true;
      return;
    }

    // Subsequent runs: skip if this is just the echo of our own write.
    if (JSON.stringify(value) === JSON.stringify(lastWrittenValue.current)) {
      return;
    }

    // External change — rebuild the layer(s) from scratch.
    group.clearLayers();
    if (featureCollection) {
      addFCLayers();
    } else {
      const layer = buildLayer();
      if (layer) group.addLayer(layer);
    }
    lastWrittenValue.current = value;
  }, [value, pathOptions, markerIcon, valueParse, featureCollection]);

  const persist = useCallback(() => {
    const group = featureGroupRef.current;
    if (!group) return;
    const layers = group.getLayers();
    // Snapshot the current form value so `valueTransform` can carry forward
    // fields like `Feature.properties` that aren't derivable from the drawn
    // geometry alone. Read via `getValues` (not the watched `value`) so we
    // get the latest committed value at persist time, not a stale render.
    const prev = form.getValues(source);
    const write = (geom: GeoJSON.Geometry | null) => {
      const stored = valueTransform && geom ? valueTransform(geom, prev) : geom;
      form.setValue(source, stored, { shouldDirty: true });
      lastWrittenValue.current = stored;
    };
    if (featureCollection) {
      // FeatureCollection mode: emit one Feature per layer. Preserve
      // properties from the previous form value by index — best-effort, since
      // layer order can shift when the user removes a feature.
      const prevFC =
        (prev as GeoJSON.FeatureCollection | null | undefined)?.type ===
        "FeatureCollection"
          ? (prev as GeoJSON.FeatureCollection)
          : null;
      const features: GeoJSON.Feature[] = [];
      layers.forEach((l, i) => {
        const geometry = layerToGeometry(l);
        if (!geometry) return;
        features.push({
          type: "Feature",
          geometry,
          properties: prevFC?.features?.[i]?.properties ?? {},
        });
      });
      const fc: GeoJSON.FeatureCollection = {
        type: "FeatureCollection",
        features,
      };
      form.setValue(source, fc, { shouldDirty: true });
      lastWrittenValue.current = fc;
      return;
    }
    if (layers.length === 0) {
      form.setValue(source, null, { shouldDirty: true });
      lastWrittenValue.current = null;
      return;
    }
    if (collection) {
      const geometries = layers
        .map((l) => layerToGeometry(l))
        .filter((g): g is GeoJSON.Geometry => g !== null);
      write({ type: "GeometryCollection", geometries });
      return;
    }
    if (multi) {
      // Combine into Multi* geometry of the configured shape kind.
      const geometries = layers
        .map((l) => layerToGeometry(l))
        .filter((g): g is GeoJSON.Geometry => g !== null);
      const combined = combineMulti(shape, geometries);
      if (validate && combined && validate(combined)) {
        // Validation failed — leave previous value untouched.
        return;
      }
      write(combined);
      return;
    }
    // Single feature — use the first/most-recent layer.
    const geom = layerToGeometry(layers[layers.length - 1]);
    if (validate && geom && validate(geom)) return;
    write(geom);
  }, [
    collection,
    featureCollection,
    multi,
    shape,
    source,
    form,
    validate,
    valueTransform,
  ]);

  const handleCreate = useCallback<PM.CreateEventHandler>(
    (e) => {
      const group = featureGroupRef.current;
      if (!group) return;
      // The drawn layer is already added to the FeatureGroup by the package
      // (via globalOptions.layerGroup). For single-feature shapes, remove all
      // OTHER layers so the new draw replaces the previous geometry.
      // FeatureCollection mode keeps every drawn layer (one Feature each).
      if (!multi && !collection && !featureCollection) {
        const newLayer = e.layer as L.Layer;
        group.eachLayer((l) => {
          if (l !== newLayer) group.removeLayer(l);
        });
      }
      persist();
    },
    [collection, featureCollection, multi, persist],
  );

  const handleUpdate = useCallback<PM.UpdateEventHandler>(
    () => persist(),
    [persist],
  );
  const handleLayerRemove = useCallback<PM.RemoveEventHandler>(
    () => persist(),
    [persist],
  );
  // The package separates `pm:cut` on the map (onMapCut) from `pm:cut` on the
  // source layer (onLayerCut). The map-level event fires once per cut operation
  // and is the one we use to persist. The layer-level event would fire again
  // for the cut layer; ignoring it avoids the dedup hack the in-house
  // GeomanEvents needed.
  const handleMapCut = useCallback<PM.CutEventHandler>(
    () => persist(),
    [persist],
  );
  const handleLayerCut = useCallback<PM.CutEventHandler>(() => undefined, []);

  return {
    featureGroupRef,
    geomanControlsProps: {
      onCreate: handleCreate,
      onUpdate: handleUpdate,
      onLayerRemove: handleLayerRemove,
      onMapCut: handleMapCut,
      onLayerCut: handleLayerCut,
    },
  };
};

const combineMulti = (
  shape: ShapeKind,
  geoms: GeoJSON.Geometry[],
): GeoJSON.Geometry | null => {
  if (geoms.length === 0) return null;
  switch (shape) {
    case "MultiPoint":
      return {
        type: "MultiPoint",
        coordinates: geoms
          .filter((g): g is GeoJSON.Point => g.type === "Point")
          .map((g) => g.coordinates),
      };
    case "MultiLineString":
      return {
        type: "MultiLineString",
        coordinates: geoms
          .filter((g): g is GeoJSON.LineString => g.type === "LineString")
          .map((g) => g.coordinates),
      };
    case "MultiPolygon":
      return {
        type: "MultiPolygon",
        coordinates: geoms
          .filter((g): g is GeoJSON.Polygon => g.type === "Polygon")
          .map((g) => g.coordinates),
      };
    default:
      // Single-shape kind with multi:true — keep the most recent only.
      return geoms[geoms.length - 1];
  }
};

export { type UseGeomanRHFOptions, type UseGeomanRHFReturn, useGeomanRHF };
