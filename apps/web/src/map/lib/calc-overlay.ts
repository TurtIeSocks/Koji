export interface RouteSegment {
  source: [number, number];
  target: [number, number];
  length: number;
}

/** Ordered [lon,lat] coords from a calc result — the route/cluster centers live in
 *  the first feature's MultiPoint (or a LineString). */
export function routeCoords(fc: GeoJSON.FeatureCollection | null | undefined): [number, number][] {
  const g = fc?.features[0]?.geometry;
  if (g && (g.type === "MultiPoint" || g.type === "LineString")) {
    return g.coordinates as [number, number][];
  }
  return [];
}

/** Consecutive segments of the ordered route. `length` is a cos(lat)-corrected
 *  planar distance in degrees — good enough to RANK segments for coloring (viz only). */
export function routeSegments(coords: [number, number][]): RouteSegment[] {
  const segs: RouteSegment[] = [];
  for (let i = 0; i < coords.length - 1; i++) {
    const a = coords[i];
    const b = coords[i + 1];
    const midLat = (((a[1] + b[1]) / 2) * Math.PI) / 180;
    const dx = (b[0] - a[0]) * Math.cos(midLat);
    const dy = b[1] - a[1];
    segs.push({ source: a, target: b, length: Math.hypot(dx, dy) });
  }
  return segs;
}

/** Green → yellow → red ramp for t in [0,1] (short leg → long leg). */
export function rampColor(t: number): [number, number, number] {
  const c = Math.max(0, Math.min(1, t));
  const r = c < 0.5 ? Math.round(510 * c) : 255;
  const g = c < 0.5 ? 255 : Math.round(510 * (1 - c));
  return [r, g, 0];
}

/** Per-segment color: the longest legs go red, the shortest green — normalized
 *  across THIS route's own segments (the old map's "spot the long jumps" cue). */
export function segmentColors(segs: RouteSegment[]): [number, number, number][] {
  if (segs.length === 0) return [];
  const lens = segs.map((s) => s.length);
  const min = Math.min(...lens);
  const max = Math.max(...lens);
  const span = max - min;
  return segs.map((s) => rampColor(span > 0 ? (s.length - min) / span : 0));
}
