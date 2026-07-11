/** UI calc modes. "cluster" maps to the backend `route` mode (cluster + route)
 *  — we always produce an ordered route now; `sortBy` chooses the ordering
 *  (or None). "bootstrap" maps to backend `bootstrap`. */
export type CalcMode = "cluster" | "bootstrap";

/** Clustering/bootstrap strategy → backend `calculationMode`. */
export type CalcStrategy = "radius" | "s2";

export type TthFilter = "All" | "Known" | "Unknown";

export interface CalcParams {
  mode: CalcMode;
  strategy: CalcStrategy;
  /** meters (radius strategy). */
  radius: number;
  /** S2 strategy. */
  s2Level: number;
  s2Size: number;
  minPoints: number;
  /** clustering algorithm (`clustering.mode`) — honeycomb/fastest/fast/balanced/
   *  better/best from GET /algorithms. null = server default (balanced). */
  clusterMode?: string | null;
  /** `clustering.maxClusters`. null/0 = unlimited. Radius strategy only. */
  maxClusters?: number | null;
  /** `clustering.centerClusters` — smallest-enclosing-circle recentre. Radius only. */
  centerClusters: boolean;
  /** routing algorithm (`routing.sortBy`). null = server default (TSP for route). */
  sortBy?: string | null;
  /** spawnpoint confirmed/unconfirmed filter (`dataFilter.tth`). */
  tth: TthFilter;
}

/** Wrap any feature/geometry as a single-feature FeatureCollection area. */
export function featureToAreaFC(feature: GeoJSON.Feature): GeoJSON.FeatureCollection {
  return { type: "FeatureCollection", features: [feature] };
}

/** A selected route's geojson coords ([lon,lat]) → koji SingleVec ([lat,lon]). */
export function routeCoordsToClusters(feature: GeoJSON.Feature | null): [number, number][] {
  const geom = feature?.geometry;
  let ring: number[][] = [];
  if (geom?.type === "LineString" || geom?.type === "MultiPoint") ring = geom.coordinates;
  else if (geom?.type === "Polygon") ring = geom.coordinates[0] ?? [];
  return ring.map(([lon, lat]) => [lat, lon]);
}

export interface CalcInputs {
  /** geojson area for cluster/bootstrap. */
  area?: GeoJSON.FeatureCollection;
  /** golbat data category (derived from the route's mode). */
  category?: string;
}

/** Build the POST /api/v2/jobs body from the panel params + resolved inputs.
 *  Field names are the v2 backend wire names (camelCase). */
export function buildCalcBody(p: CalcParams, inp: CalcInputs): Record<string, unknown> {
  const routing = p.sortBy ? { routing: { sortBy: p.sortBy } } : {};

  if (p.mode === "bootstrap") {
    const bootstrap: Record<string, unknown> = { calculationMode: p.strategy, radius: p.radius };
    if (p.strategy === "s2") {
      bootstrap.s2Level = p.s2Level;
      bootstrap.s2Size = p.s2Size;
    }
    return { mode: "bootstrap", category: inp.category, area: inp.area, bootstrap, ...routing };
  }

  // "cluster" → backend `route` mode: cluster then order into a route.
  const clustering: Record<string, unknown> = {
    calculationMode: p.strategy,
    minPoints: p.minPoints,
  };
  if (p.strategy === "s2") {
    clustering.s2Level = p.s2Level;
    clustering.s2Size = p.s2Size;
  } else {
    clustering.radius = p.radius;
    if (p.clusterMode) clustering.mode = p.clusterMode;
    if (p.maxClusters && p.maxClusters > 0) clustering.maxClusters = p.maxClusters;
    if (p.centerClusters) clustering.centerClusters = true;
  }
  const dataFilter = p.tth && p.tth !== "All" ? { dataFilter: { tth: p.tth } } : {};
  return {
    mode: "route",
    category: inp.category,
    area: inp.area,
    clustering,
    ...routing,
    ...dataFilter,
  };
}

export interface RouteStatsInputs {
  /** Golbat points `[lat, lon]` the route is meant to cover. */
  dataPoints: [number, number][];
  /** The route's ordered cluster centers `[lat, lon]`. */
  clusters: [number, number][];
  /** Coverage radius in meters (the calc panel's current radius). */
  radius: number;
  minPoints: number;
}

/** POST /api/v2/jobs body for the `routeStats` op — stats for an EXISTING route
 *  (its `clusters`) against its `dataPoints`, with NO re-clustering. Lets a
 *  loaded route report coverage/score/distance without recomputing the route. */
export function buildRouteStatsBody(inp: RouteStatsInputs): Record<string, unknown> {
  return {
    mode: "routeStats",
    dataPoints: inp.dataPoints,
    clusters: inp.clusters,
    radius: inp.radius,
    minPoints: inp.minPoints,
  };
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
