import { useCallback, useEffect, useRef, useState } from "react";
import { useFormContext, useWatch } from "react-hook-form";
import { shouldCommitEdit, explodeSplitFeatures } from "@/map/lib/edit-serialize";
import type { DrawMode } from "@/map/lib/edit-modes";

const EMPTY_FC: GeoJSON.FeatureCollection = { type: "FeatureCollection", features: [] };

/** Default: the form value IS a single geometry (or Feature) → one draft feature. */
function defaultToFeatures(value: unknown): GeoJSON.Feature[] {
  if (value == null) return [];
  const v = value as GeoJSON.Feature | GeoJSON.Geometry;
  if ((v as GeoJSON.Feature).type === "Feature") return [v as GeoJSON.Feature];
  return [{ type: "Feature", geometry: v as GeoJSON.Geometry, properties: {} }];
}
function defaultFromFeatures(features: GeoJSON.Feature[]): unknown {
  return features[0]?.geometry ?? null;
}

export interface UseDeckEditRHFOptions {
  source: string;
  /** Map the form value → the draft features to edit (default: one feature).
   *  Return several features to edit a multi-part geometry (e.g. a MultiPolygon
   *  split into one editable Polygon feature per part). */
  toFeatures?: (value: unknown) => GeoJSON.Feature[];
  /** Map the edited features → the stored form value (default: first geometry). */
  fromFeatures?: (features: GeoJSON.Feature[], prev: unknown) => unknown;
}

export interface UseDeckEditRHFReturn {
  draft: GeoJSON.FeatureCollection;
  mode: DrawMode;
  setMode: (m: DrawMode) => void;
  selectedIndexes: number[];
  onEdit: (e: {
    updatedData: GeoJSON.FeatureCollection;
    editType?: string;
    editContext?: { featureIndexes?: number[] };
  }) => void;
  onSelect: (indexes: number[]) => void;
}

export function useDeckEditRHF({
  source,
  toFeatures = defaultToFeatures,
  fromFeatures = defaultFromFeatures,
}: UseDeckEditRHFOptions): UseDeckEditRHFReturn {
  const form = useFormContext();
  const value = useWatch({ name: source });
  const [draft, setDraft] = useState<GeoJSON.FeatureCollection>(EMPTY_FC);
  const [mode, setMode] = useState<DrawMode>("none");
  const [selectedIndexes, setSelectedIndexes] = useState<number[]>([]);
  const lastWritten = useRef<unknown>(undefined);
  const hydrated = useRef(false);
  const autoInited = useRef(false);

  // Hydrate from the form value; skip the echo of our own writes.
  useEffect(() => {
    if (hydrated.current && JSON.stringify(value) === JSON.stringify(lastWritten.current)) return;
    const features = toFeatures(value);
    setDraft(features.length ? { type: "FeatureCollection", features } : EMPTY_FC);
    lastWritten.current = value;
    hydrated.current = true;
    // With an existing shape, default straight into modify + select every part so
    // it's editable immediately — no "click the shape first". Once only; after
    // that the user owns the mode/selection.
    if (!autoInited.current && features.length) {
      autoInited.current = true;
      setMode("modify");
      setSelectedIndexes(features.map((_, i) => i));
    }
  }, [value, toFeatures]);

  const commit = useCallback(
    (fc: GeoJSON.FeatureCollection) => {
      setDraft(fc);
      const prev = form.getValues(source) as unknown;
      const stored = fromFeatures(fc.features, prev);
      lastWritten.current = stored;
      form.setValue(source, stored, { shouldDirty: true });
    },
    [form, source, fromFeatures],
  );

  const onEdit = useCallback(
    (e: {
      updatedData: GeoJSON.FeatureCollection;
      editType?: string;
      editContext?: { featureIndexes?: number[] };
    }) => {
      if (!shouldCommitEdit(e.editType)) {
        setDraft(e.updatedData);
        return;
      }
      if (e.editType === "split" && e.editContext?.featureIndexes) {
        commit(explodeSplitFeatures(e.updatedData, e.editContext.featureIndexes));
      } else {
        commit(e.updatedData);
      }
    },
    [commit],
  );

  return { draft, mode, setMode, selectedIndexes, onEdit, onSelect: setSelectedIndexes };
}
