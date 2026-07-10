import { useCallback, useEffect, useRef, useState } from "react";
import { useFormContext, useWatch } from "react-hook-form";
import { shouldCommitEdit, explodeSplitFeatures } from "@/map/lib/edit-serialize";
import type { DrawMode } from "@/map/lib/edit-modes";

const EMPTY_FC: GeoJSON.FeatureCollection = { type: "FeatureCollection", features: [] };

function defaultToFeature(value: unknown): GeoJSON.Feature | null {
  if (value == null) return null;
  const v = value as GeoJSON.Feature | GeoJSON.Geometry;
  if ((v as GeoJSON.Feature).type === "Feature") return v as GeoJSON.Feature;
  return { type: "Feature", geometry: v as GeoJSON.Geometry, properties: {} };
}
function defaultFromFeatures(features: GeoJSON.Feature[]): unknown {
  return features[0]?.geometry ?? null;
}

export interface UseDeckEditRHFOptions {
  source: string;
  /** How the form value maps to/from the draft FeatureCollection. */
  toFeature?: (value: unknown) => GeoJSON.Feature | null;
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
  toFeature = defaultToFeature,
  fromFeatures = defaultFromFeatures,
}: UseDeckEditRHFOptions): UseDeckEditRHFReturn {
  const form = useFormContext();
  const value = useWatch({ name: source });
  const [draft, setDraft] = useState<GeoJSON.FeatureCollection>(EMPTY_FC);
  const [mode, setMode] = useState<DrawMode>("none");
  const [selectedIndexes, setSelectedIndexes] = useState<number[]>([]);
  const lastWritten = useRef<unknown>(undefined);
  const hydrated = useRef(false);

  // Hydrate from the form value; skip the echo of our own writes.
  useEffect(() => {
    if (hydrated.current && JSON.stringify(value) === JSON.stringify(lastWritten.current)) return;
    const feat = toFeature(value);
    setDraft(feat ? { type: "FeatureCollection", features: [feat] } : EMPTY_FC);
    lastWritten.current = value;
    hydrated.current = true;
  }, [value, toFeature]);

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
