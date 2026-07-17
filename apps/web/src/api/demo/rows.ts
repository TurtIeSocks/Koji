// Shared row builders + a monotonic demo clock, used by BOTH the demo CRUD
// provider (create/update) and the bulk-import commit path so a geofence/route
// row is constructed identically no matter which surface writes it.

import type { GeofenceRow, RouteRow } from "./db";
import { featureBbox } from "./seeds/geometry";

// ── Monotonic demo clock ────────────────────────────────────────────────
// `updated_at` must strictly increase on every write so ordering is stable
// even when two writes land in the same millisecond (or a test freezes the
// wall clock). Defaults to Date.now(); injectable for deterministic tests.
let clock = 0;
let nowMs: () => number = () => Date.now();

/** Override the wall-clock source (tests). */
export function setDemoNow(fn: () => number): void {
  nowMs = fn;
}

/** A strictly-increasing ISO timestamp for a write's `updated_at`. */
export function nowIso(): string {
  const t = Math.max(nowMs(), clock + 1);
  clock = t;
  return new Date(t).toISOString();
}

// ── Geometry helpers ────────────────────────────────────────────────────

/** GeoJSON geometry type, defaulting to Polygon for a missing geometry. */
export function geoTypeOf(geometry?: GeoJSON.Geometry | null): string {
  return geometry?.type ?? "Polygon";
}

/** Recompute a geofence's stored AABB from its geometry. Non-areal or missing
 *  geometry → all-null bbox (never intersects a viewport query). */
export function bboxFields(geometry?: GeoJSON.Geometry | null): {
  min_lat: number | null;
  min_lng: number | null;
  max_lat: number | null;
  max_lng: number | null;
} {
  if (geometry && (geometry.type === "Polygon" || geometry.type === "MultiPolygon")) {
    const b = featureBbox({ type: "Feature", geometry, properties: {} });
    return { min_lat: b.minLat, min_lng: b.minLon, max_lat: b.maxLat, max_lng: b.maxLon };
  }
  return { min_lat: null, min_lng: null, max_lat: null, max_lng: null };
}

/** Count the positions in any GeoJSON geometry (a route's `points`). Walks the
 *  nested `coordinates` arrays down to the innermost `[lon, lat]` pairs. */
export function countPositions(geometry?: GeoJSON.Geometry | null): number {
  if (!geometry) return 0;
  if (geometry.type === "GeometryCollection") {
    return geometry.geometries.reduce((sum, g) => sum + countPositions(g), 0);
  }
  const walk = (a: unknown): number => {
    if (!Array.isArray(a)) return 0;
    if (typeof a[0] === "number") return 1; // a single position
    return a.reduce((sum: number, x) => sum + walk(x), 0);
  };
  return walk((geometry as { coordinates?: unknown }).coordinates);
}

// ── Row builders ────────────────────────────────────────────────────────
// `data` is the write payload (already property-normalised by
// serializeGeofenceWrite where relevant); `existing` is the previous row on an
// update (null on create), letting a partial patch merge over it.

type WriteData = Record<string, unknown>;

export function buildGeofenceRow(
  data: WriteData,
  existing: GeofenceRow | null,
  id: number,
  now: string,
): GeofenceRow {
  const merged = { ...(existing ?? {}), ...data } as WriteData;
  const geometry = merged.geometry as GeoJSON.Geometry | undefined;
  return {
    id,
    name: String(merged.name ?? ""),
    mode: String(merged.mode ?? "unset"),
    parent: (merged.parent as number | null | undefined) ?? null,
    geo_type: geoTypeOf(geometry),
    // db.ts types geometry as Polygon; MultiPolygon geofences store fine here.
    geometry: geometry as GeoJSON.Polygon,
    ...bboxFields(geometry),
    projects: Array.isArray(merged.projects) ? (merged.projects as unknown[]).map(Number) : [],
    properties: Array.isArray(merged.properties) ? (merged.properties as unknown[]) : [],
    created_at: existing?.created_at ?? now,
    updated_at: now,
  };
}

export function buildRouteRow(
  data: WriteData,
  existing: RouteRow | null,
  id: number,
  now: string,
): RouteRow {
  const merged = { ...(existing ?? {}), ...data } as WriteData;
  const geometry = merged.geometry as GeoJSON.Geometry;
  return {
    id,
    geofence_id: Number(merged.geofence_id ?? 0),
    name: String(merged.name ?? ""),
    description: (merged.description as string | null | undefined) ?? null,
    mode: String(merged.mode ?? "unset"),
    geometry,
    points: countPositions(geometry),
    created_at: existing?.created_at ?? now,
    updated_at: now,
  };
}
