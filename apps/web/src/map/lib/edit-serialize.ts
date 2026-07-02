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

/** Ray-casting point-in-ring test (ring is a closed [lon,lat] loop). */
function pointInRing(pt: GeoJSON.Position, ring: GeoJSON.Position[]): boolean {
  const [x, y] = pt;
  let inside = false;
  for (let i = 0, j = ring.length - 1; i < ring.length; j = i++) {
    const [xi, yi] = ring[i];
    const [xj, yj] = ring[j];
    if ((yi > y) !== (yj > y) && x < ((xj - xi) * (y - yi)) / (yj - yi) + xi) inside = !inside;
  }
  return inside;
}

/** Re-nest a split MultiPolygon into proper polygons. SplitPolygonMode flattens
 *  EVERY ring — including interior (hole) rings — into its own top-level polygon,
 *  so a hole that wasn't crossed by the split line becomes a solid polygon. Detect
 *  rings contained within another and re-attach them as interior rings.
 *  ponytail: 1-level containment only; a hole-within-a-hole stays standalone. */
function renestSplitPolygons(mp: GeoJSON.MultiPolygon): GeoJSON.Polygon[] {
  const rings = mp.coordinates.map((poly) => poly[0]).filter((r): r is GeoJSON.Position[] => !!r);
  // container[i] = index of the ring that contains ring i (a vertex of it), or -1.
  const container = rings.map((ring, i) =>
    rings.findIndex((other, j) => j !== i && ring[0] != null && pointInRing(ring[0], other)),
  );
  const polys = new Map<number, GeoJSON.Position[][]>(); // top-level ring index → its rings
  rings.forEach((ring, i) => {
    if (container[i] === -1) polys.set(i, [ring]);
  });
  rings.forEach((ring, i) => {
    if (container[i] === -1) return;
    const parent = polys.get(container[i]);
    if (parent) parent.push(ring); // interior ring → hole of its container
    else polys.set(i, [ring]); // container was itself a hole → keep standalone
  });
  return [...polys.values()].map((coordinates) => ({ type: "Polygon", coordinates }));
}

/** editable-layers' SplitPolygonMode replaces the split polygon with ONE
 *  MultiPolygon feature (both halves + a gap), so they select/move together.
 *  Explode it into INDEPENDENT Polygon features — the expected "split into two
 *  separate shapes" semantics — while preserving any holes (see renestSplitPolygons).
 *  Other features (and non-MultiPolygon geometries) pass through untouched. */
export function explodeSplitFeatures(
  fc: GeoJSON.FeatureCollection,
  featureIndexes: number[],
): GeoJSON.FeatureCollection {
  const split = new Set(featureIndexes);
  const features: GeoJSON.Feature[] = [];
  fc.features.forEach((f, i) => {
    if (split.has(i) && f.geometry?.type === "MultiPolygon") {
      for (const geometry of renestSplitPolygons(f.geometry as GeoJSON.MultiPolygon)) {
        features.push({ ...f, geometry });
      }
    } else {
      features.push(f);
    }
  });
  return { type: "FeatureCollection", features };
}
