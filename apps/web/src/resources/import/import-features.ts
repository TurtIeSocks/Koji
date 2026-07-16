/** Bridges the import wizard's RHF `features` rows (GeoJSON features carrying
 *  top-level assignment fields — see to-import-items.ts FormFeature) to
 *  useDeckEditRHF's editable Feature[] and back. Rows are matched by a
 *  self-contained `_row` payload (the ENTIRE row) embedded in properties at
 *  stamp time — never by an index into external state — so a mid-list delete,
 *  a reorder, or an edit AFTER a delete can never mis-assign or drop
 *  name/mode/projects. A feature with no `_row` (freshly drawn in step 2)
 *  becomes a new row. */

interface ImportRow {
  geometry?: unknown;
  properties?: Record<string, unknown> | null;
  [key: string]: unknown;
}

export function importToFeatures(value: unknown): GeoJSON.Feature[] {
  const rows = (value as ImportRow[]) ?? [];
  return rows.map((r) => {
    // Strip any incoming `_row` key so imported source data can't collide
    // with our stamp (we own this key).
    const { _row: _ignore, ...props } = (r.properties ?? {}) as Record<string, unknown>;
    return {
      type: "Feature" as const,
      geometry: r.geometry as GeoJSON.Geometry,
      // Self-contained row payload: survives any delete/reorder because it
      // never indexes into an external array. `prev` shrinks on delete while
      // hydrate-time index stamps don't renumber — an index-based stamp
      // (`_rowIdx`, the first design) dropped assignments on the first edit
      // AFTER a delete.
      properties: { ...props, _row: r },
    };
  });
}

export function importFromFeatures(feats: GeoJSON.Feature[], _prev: unknown): unknown {
  return feats.map((f) => {
    const { _row, ...properties } = (f.properties ?? {}) as Record<string, unknown>;
    const base = _row as ImportRow | undefined;
    return base
      ? { ...base, geometry: f.geometry, properties }
      : { type: "Feature", geometry: f.geometry, properties };
  });
}
