export type CalcMode = "cluster" | "route" | "bootstrap" | "reroute" | "routeStats";

/** Modes that compute over an AREA + golbat category (vs. re-processing a route). */
export const AREA_MODES: CalcMode[] = ["cluster", "route", "bootstrap"];
/** Modes that take an existing route's points as input (`clusters`). */
export const ROUTE_INPUT_MODES: CalcMode[] = ["reroute", "routeStats"];

export interface CalcParams {
  mode: CalcMode;
  /** golbat data category (cluster/route/bootstrap). */
  category: string;
  radius: number;
  minPoints: number;
  /** clustering algorithm from GET /algorithms (cluster/route); server default if unset. */
  clusterMode?: string | null;
}

/** Wrap any feature/geometry as a single-feature FeatureCollection area. */
export function featureToAreaFC(feature: GeoJSON.Feature): GeoJSON.FeatureCollection {
  return { type: "FeatureCollection", features: [feature] };
}

/** A selected route's geojson coords ([lon,lat]) → koji SingleVec ([lat,lon]).
 *  reroute/routeStats consume `clusters` in koji [lat,lon] order, NOT geojson. */
export function routeCoordsToClusters(feature: GeoJSON.Feature | null): [number, number][] {
  const geom = feature?.geometry;
  let ring: number[][] = [];
  if (geom?.type === "LineString" || geom?.type === "MultiPoint") ring = geom.coordinates;
  else if (geom?.type === "Polygon") ring = geom.coordinates[0] ?? [];
  return ring.map(([lon, lat]) => [lat, lon]);
}

export interface CalcInputs {
  /** geojson area for cluster/route/bootstrap. */
  area?: GeoJSON.FeatureCollection;
  /** [lat,lon] clusters for reroute/routeStats. */
  clusters?: [number, number][];
}

/** Build the POST /api/v2/jobs body. `routing` is omitted so the server applies
 *  its per-mode default (route → TSP); `clustering.mode` rides only when chosen. */
export function buildCalcBody(p: CalcParams, inp: CalcInputs): Record<string, unknown> {
  switch (p.mode) {
    case "cluster":
    case "route": {
      const clustering: Record<string, unknown> = { radius: p.radius, minPoints: p.minPoints };
      if (p.clusterMode) clustering.mode = p.clusterMode;
      return { mode: p.mode, category: p.category, area: inp.area, clustering };
    }
    case "bootstrap":
      return { mode: "bootstrap", category: p.category, area: inp.area, bootstrap: { radius: p.radius } };
    case "reroute":
      return { mode: "reroute", clusters: inp.clusters ?? [], radius: p.radius };
    case "routeStats":
      return { mode: "routeStats", clusters: inp.clusters ?? [], radius: p.radius, minPoints: p.minPoints };
  }
}

export interface CalcResult {
  fc: GeoJSON.FeatureCollection | null;
  stats: unknown;
}

/** Extract the geojson FeatureCollection + stats from a succeeded job record. */
export function parseCalcResult(
  record: { result?: { data?: unknown; stats?: unknown } | null } | null | undefined,
): CalcResult {
  const data = record?.result?.data;
  const isFC =
    !!data && typeof data === "object" && (data as { type?: string }).type === "FeatureCollection";
  return { fc: isFC ? (data as GeoJSON.FeatureCollection) : null, stats: record?.result?.stats ?? null };
}
