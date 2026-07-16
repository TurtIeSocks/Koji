/** Bridges the import wizard's RHF `features` rows (GeoJSON features carrying
 *  top-level assignment fields — see to-import-items.ts FormFeature) to
 *  useDeckEditRHF's editable Feature[] and back. Rows are matched by a
 *  `_rowIdx` property stamped on the way out, so a mid-list delete or a
 *  reorder inside the edit layer can never mis-assign name/mode/projects.
 *  A feature with no `_rowIdx` (freshly drawn in step 2) becomes a new row. */

interface ImportRow {
  geometry?: unknown;
  properties?: Record<string, unknown> | null;
  [key: string]: unknown;
}

export function importToFeatures(value: unknown): GeoJSON.Feature[] {
  const rows = (value as ImportRow[]) ?? [];
  return rows.map((r, i) => ({
    type: "Feature",
    geometry: r.geometry as GeoJSON.Geometry,
    properties: { ...(r.properties ?? {}), _rowIdx: i },
  }));
}

export function importFromFeatures(feats: GeoJSON.Feature[], prev: unknown): unknown {
  const rows = (prev as ImportRow[]) ?? [];
  return feats.map((f) => {
    const { _rowIdx, ...properties } = (f.properties ?? {}) as Record<string, unknown>;
    const base = typeof _rowIdx === "number" ? rows[_rowIdx] : undefined;
    return base
      ? { ...base, geometry: f.geometry, properties }
      : { type: "Feature", geometry: f.geometry, properties };
  });
}
