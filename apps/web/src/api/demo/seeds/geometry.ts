// Pure geometry helpers for the demo world's synthetic marker generation.
//
// GeoJSON convention: coordinates are `[lon, lat]`. Koji's own domain
// (points passed to/from the map, koji-core, etc.) uses `[lat, lon]`. Every
// function here spells out which convention it's using in its signature —
// a bare `[number, number]` parameter is always documented as one or the
// other — because silently swapping the two is Koji's classic bug.

/** Axis-aligned bounding box in lat/lon degrees. */
export interface Bbox {
  minLat: number;
  minLon: number;
  maxLat: number;
  maxLon: number;
}

/** Ray-cast point-in-polygon test (even-odd rule), run across every ring in
 *  `polygonCoords` (outer boundary + any holes) — a point inside a hole
 *  flips parity back to "outside" for free, no separate hole handling
 *  needed.
 *
 *  `point` is `[lat, lon]` (Koji convention). `polygonCoords` is a GeoJSON
 *  `Polygon.coordinates` value: rings of `[lon, lat]` positions. */
export function pointInPolygon(point: [number, number], polygonCoords: GeoJSON.Position[][]): boolean {
  const [lat, lon] = point;
  let inside = false;
  for (const ring of polygonCoords) {
    for (let i = 0, j = ring.length - 1; i < ring.length; j = i++) {
      const [xi, yi] = ring[i]; // xi = lon, yi = lat
      const [xj, yj] = ring[j];
      const crosses = yi > lat !== yj > lat && lon < ((xj - xi) * (lat - yi)) / (yj - yi) + xi;
      if (crosses) inside = !inside;
    }
  }
  return inside;
}

/** Bounding box of a GeoJSON `Polygon.coordinates` value (every ring, so a
 *  hole never shrinks it — holes are always inside the outer ring anyway). */
export function polygonBbox(coords: GeoJSON.Position[][]): Bbox {
  let minLat = Number.POSITIVE_INFINITY;
  let minLon = Number.POSITIVE_INFINITY;
  let maxLat = Number.NEGATIVE_INFINITY;
  let maxLon = Number.NEGATIVE_INFINITY;
  for (const ring of coords) {
    for (const [lon, lat] of ring) {
      if (lat < minLat) minLat = lat;
      if (lat > maxLat) maxLat = lat;
      if (lon < minLon) minLon = lon;
      if (lon > maxLon) maxLon = lon;
    }
  }
  return { minLat, minLon, maxLat, maxLon };
}

const EMPTY_BBOX: Bbox = {
  minLat: Number.POSITIVE_INFINITY,
  minLon: Number.POSITIVE_INFINITY,
  maxLat: Number.NEGATIVE_INFINITY,
  maxLon: Number.NEGATIVE_INFINITY,
};

function mergeBbox(a: Bbox, b: Bbox): Bbox {
  return {
    minLat: Math.min(a.minLat, b.minLat),
    minLon: Math.min(a.minLon, b.minLon),
    maxLat: Math.max(a.maxLat, b.maxLat),
    maxLon: Math.max(a.maxLon, b.maxLon),
  };
}

/** Bounding box of a Polygon or MultiPolygon feature's geometry. The demo's
 *  nyc-areas.geo.json seed is all single-ring Polygons; MultiPolygon is
 *  handled defensively for any future seed. */
export function featureBbox(feature: GeoJSON.Feature): Bbox {
  const geometry = feature.geometry;
  if (geometry.type === "Polygon") return polygonBbox(geometry.coordinates);
  if (geometry.type === "MultiPolygon") {
    return geometry.coordinates.map(polygonBbox).reduce(mergeBbox, EMPTY_BBOX);
  }
  throw new Error(`featureBbox: unsupported geometry type "${geometry.type}"`);
}
