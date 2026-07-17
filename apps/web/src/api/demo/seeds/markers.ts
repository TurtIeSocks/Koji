// Deterministic synthetic marker generation for the demo world. Every
// category is generated once per NYC-areas polygon, from a seed derived
// purely from (feature index, category name) — no Date.now(), no shared
// mutable counters — so two `generateMarkers(fc)` calls on the same
// FeatureCollection produce byte-identical results (see markers.test.ts,
// "is deterministic").
//
// GeoJSON convention: ring/geometry coordinates are `[lon, lat]`. Koji's
// own point convention (and this module's public `query()` return value) is
// `[lat, lon]`. Internal helpers spell out `lat`/`lon` by name rather than
// passing an ambiguous tuple, to avoid Koji's classic lat/lon mixup right
// at this kind of geometry boundary.

import { featureBbox, pointInPolygon } from "./geometry";
import type { Bbox } from "./geometry";
import { mulberry32 } from "./prng";

/** Frozen "now" for demo data generation — deliberately NOT `Date.now()`,
 *  so generated spawnpoint `updatedAt` values (and the seeded row
 *  timestamps built from it in seed.ts) stay byte-identical across every
 *  run: local dev, CI, and the built demo site alike. */
export const DEMO_EPOCH_MS = Date.UTC(2026, 6, 1); // 2026-07-01T00:00:00Z

const THIRTY_DAYS_MS = 30 * 24 * 60 * 60 * 1000;
const KERNEL_SIGMA_DEG = 0.0015;
const MAX_REJECTION_ATTEMPTS = 60;
const MAX_KERNEL_REJECTION_ATTEMPTS = 30;
const TTH_KNOWN_RATE = 0.6;
const CLUSTER_UNIFORM_SHARE = 0.2;

/** Average marker count per polygon, per category. Spawnpoints cluster
 *  (see `generatePoints`); everything else scatters uniformly. */
const CATEGORY_DENSITY: Record<string, number> = {
  spawnpoint: 600,
  pokestop: 120,
  gym: 40,
  station: 15,
};

export type TthFilter = "All" | "Known" | "Unknown";

export interface MarkerQueryOptions {
  bbox?: Bbox;
  areaFeatures?: GeoJSON.Feature[];
  lastSeen?: number;
  tth?: TthFilter;
}

export interface MarkerStore {
  query(category: string, opts: MarkerQueryOptions): [number, number][];
}

interface Marker {
  lat: number;
  lon: number;
  /** Spawnpoints only. */
  updatedAt?: number;
  /** Spawnpoints only. */
  tthKnown?: boolean;
}

/** FNV-1a-ish 32-bit hash of `(featureIndex, category)` → a mulberry32
 *  seed. Any change to either input changes the whole derived point cloud;
 *  the same pair always reproduces the same seed. */
function seedFor(featureIndex: number, category: string): number {
  let h = (0x811c9dc5 ^ featureIndex) >>> 0;
  for (let i = 0; i < category.length; i++) {
    h = Math.imul(h ^ category.charCodeAt(i), 0x01000193);
  }
  return h >>> 0;
}

function ringsOf(geometry: GeoJSON.Geometry): GeoJSON.Position[][][] {
  if (geometry.type === "Polygon") return [geometry.coordinates];
  if (geometry.type === "MultiPolygon") return geometry.coordinates;
  return [];
}

function insideGeometry(lat: number, lon: number, geometry: GeoJSON.Geometry): boolean {
  return ringsOf(geometry).some((coords) => pointInPolygon([lat, lon], coords));
}

/** Rejection-samples a uniformly random point inside `geometry`, drawing
 *  lat/lon uniformly from `bbox` and retrying until it lands inside (falls
 *  back to the bbox center after `MAX_REJECTION_ATTEMPTS` — only reachable
 *  for pathologically thin/concave shapes, never Manhattan's neighborhood
 *  outlines). */
function randomPointInGeometry(rng: () => number, geometry: GeoJSON.Geometry, bbox: Bbox): [number, number] {
  for (let attempt = 0; attempt < MAX_REJECTION_ATTEMPTS; attempt++) {
    const lat = bbox.minLat + rng() * (bbox.maxLat - bbox.minLat);
    const lon = bbox.minLon + rng() * (bbox.maxLon - bbox.minLon);
    if (insideGeometry(lat, lon, geometry)) return [lat, lon];
  }
  return [(bbox.minLat + bbox.maxLat) / 2, (bbox.minLon + bbox.maxLon) / 2];
}

/** Box-Muller transform from two uniform PRNG draws → a standard-normal 2D
 *  offset scaled by `sigma` (degrees). */
function gaussianOffsetDeg(rng: () => number, sigma: number): [number, number] {
  const u1 = Math.max(rng(), Number.EPSILON); // avoid log(0)
  const u2 = rng();
  const r = Math.sqrt(-2 * Math.log(u1)) * sigma;
  const theta = 2 * Math.PI * u2;
  return [r * Math.cos(theta), r * Math.sin(theta)];
}

/** A point gaussian-scattered around kernel center `(kLat, kLon)`, rejected
 *  back to the kernel center if the offset pushed it outside `geometry`
 *  (the kernel center itself is always inside, since it came from
 *  `randomPointInGeometry`). */
function gaussianPointNear(
  rng: () => number,
  kLat: number,
  kLon: number,
  geometry: GeoJSON.Geometry,
): [number, number] {
  for (let attempt = 0; attempt < MAX_KERNEL_REJECTION_ATTEMPTS; attempt++) {
    const [dLat, dLon] = gaussianOffsetDeg(rng, KERNEL_SIGMA_DEG);
    const lat = kLat + dLat;
    const lon = kLon + dLon;
    if (insideGeometry(lat, lon, geometry)) return [lat, lon];
  }
  return [kLat, kLon];
}

/** Spawnpoints cluster: 6-12 gaussian kernels per polygon (each kernel
 *  center itself uniformly sampled inside the polygon), 80% of points
 *  gaussian-scattered around a random kernel, 20% uniformly scattered.
 *  Everything else (pokestop/gym/station) is pure uniform scatter. */
function generatePoints(
  rng: () => number,
  geometry: GeoJSON.Geometry,
  bbox: Bbox,
  count: number,
  clustered: boolean,
): [number, number][] {
  if (!clustered) {
    const points: [number, number][] = [];
    for (let i = 0; i < count; i++) points.push(randomPointInGeometry(rng, geometry, bbox));
    return points;
  }

  const kernelCount = 6 + Math.floor(rng() * 7); // 6..12 inclusive
  const kernels: [number, number][] = [];
  for (let i = 0; i < kernelCount; i++) kernels.push(randomPointInGeometry(rng, geometry, bbox));

  const uniformCount = Math.round(count * CLUSTER_UNIFORM_SHARE);
  const clusteredCount = count - uniformCount;

  const points: [number, number][] = [];
  for (let i = 0; i < clusteredCount; i++) {
    const [kLat, kLon] = kernels[Math.floor(rng() * kernelCount)];
    points.push(gaussianPointNear(rng, kLat, kLon, geometry));
  }
  for (let i = 0; i < uniformCount; i++) points.push(randomPointInGeometry(rng, geometry, bbox));
  return points;
}

function passesFilters(marker: Marker, category: string, opts: MarkerQueryOptions): boolean {
  if (opts.bbox) {
    const { minLat, minLon, maxLat, maxLon } = opts.bbox;
    if (marker.lat < minLat || marker.lat > maxLat || marker.lon < minLon || marker.lon > maxLon) return false;
  }
  if (opts.areaFeatures && opts.areaFeatures.length > 0) {
    const insideAny = opts.areaFeatures.some((f) => insideGeometry(marker.lat, marker.lon, f.geometry));
    if (!insideAny) return false;
  }
  // lastSeen/tth mirror server semantics: both apply to spawnpoints only
  // (the only category carrying updatedAt/tthKnown).
  if (category === "spawnpoint") {
    if (opts.lastSeen !== undefined && (marker.updatedAt ?? 0) < opts.lastSeen) return false;
    if (opts.tth && opts.tth !== "All") {
      const wantKnown = opts.tth === "Known";
      if (Boolean(marker.tthKnown) !== wantKnown) return false;
    }
  }
  return true;
}

/** Builds the full synthetic marker world for `fc` (every category, every
 *  polygon) once, then serves `query()` by filtering that fixed set — the
 *  generation itself never re-runs, so repeated queries against one store
 *  (and repeated `generateMarkers(fc)` calls) are always consistent. */
export function generateMarkers(fc: GeoJSON.FeatureCollection): MarkerStore {
  const byCategory: Record<string, Marker[]> = {};
  for (const category of Object.keys(CATEGORY_DENSITY)) byCategory[category] = [];

  fc.features.forEach((feature, index) => {
    const bbox = featureBbox(feature);
    for (const category of Object.keys(CATEGORY_DENSITY)) {
      const rng = mulberry32(seedFor(index, category));
      const count = CATEGORY_DENSITY[category];
      const points = generatePoints(rng, feature.geometry, bbox, count, category === "spawnpoint");
      for (const [lat, lon] of points) {
        if (category === "spawnpoint") {
          const updatedAt = DEMO_EPOCH_MS - rng() * THIRTY_DAYS_MS;
          const tthKnown = rng() < TTH_KNOWN_RATE;
          byCategory[category].push({ lat, lon, updatedAt, tthKnown });
        } else {
          byCategory[category].push({ lat, lon });
        }
      }
    }
  });

  return {
    query(category, opts) {
      const markers = byCategory[category] ?? [];
      return markers.filter((m) => passesFilters(m, category, opts)).map((m): [number, number] => [m.lat, m.lon]);
    },
  };
}
