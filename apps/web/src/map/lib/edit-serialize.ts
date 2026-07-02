export function firstGeometry(fc: GeoJSON.FeatureCollection): GeoJSON.Geometry | null {
  return fc.features[0]?.geometry ?? null;
}

/** editable-layers fires onEdit hundreds of times per polygon for the tentative
 *  cursor-follow line (updateTentativeFeature/addTentativePosition). Those don't
 *  change committed geometry and must NOT hit the store — a zustand set() per
 *  pointer-move janks the draw (measured 669 fires for a 5-vertex polygon). Only
 *  real edits commit. Load-bearing for the draw-lag fix; guarded by a unit test. */
const TENTATIVE_EDIT_TYPES = new Set(["updateTentativeFeature", "addTentativePosition"]);

export function shouldCommitEdit(editType: string | undefined): boolean {
  return !TENTATIVE_EDIT_TYPES.has(editType as string);
}

/** Every drawn feature's geometry — so Save persists ALL shapes, not just the first. */
export function allGeometries(fc: GeoJSON.FeatureCollection): GeoJSON.Geometry[] {
  return fc.features.map((f) => f.geometry).filter((g): g is GeoJSON.Geometry => g != null);
}

/** editable-layers' SplitPolygonMode replaces the split polygon with ONE
 *  MultiPolygon feature (both halves + a gap), so they select/move together.
 *  Explode the split feature(s)' MultiPolygon parts into INDEPENDENT Polygon
 *  features — the expected "split into two separate shapes" semantics. Other
 *  features (and non-MultiPolygon geometries) pass through untouched. */
export function explodeSplitFeatures(
  fc: GeoJSON.FeatureCollection,
  featureIndexes: number[],
): GeoJSON.FeatureCollection {
  const split = new Set(featureIndexes);
  const features: GeoJSON.Feature[] = [];
  fc.features.forEach((f, i) => {
    if (split.has(i) && f.geometry?.type === "MultiPolygon") {
      for (const coordinates of (f.geometry as GeoJSON.MultiPolygon).coordinates) {
        features.push({ ...f, geometry: { type: "Polygon", coordinates } });
      }
    } else {
      features.push(f);
    }
  });
  return { type: "FeatureCollection", features };
}
