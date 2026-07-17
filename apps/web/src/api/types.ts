import type { AuthProvider, DataProvider } from "ra-core";
import type { RealtimeTransport } from "@/components/realtime";

/** A job record as returned by GET /api/v2/jobs/{id} (koji-jobs JobRecord). */
export interface JobRecord {
  id: number | string;
  status: string;
  progress: number;
  phase: string | null;
  result?: { data?: unknown; stats?: unknown } | null;
  error?: string | null;
}

export interface AlgorithmOptions {
  clustering: string[];
  routing: string[];
  bootstrap: string[];
}

export interface ConfigResponse {
  start_lat: number;
  start_lon: number;
  tile_server: string;
  logged_in: boolean;
  dangerous: boolean;
  route_plugins: string[];
  clustering_plugins: string[];
  bootstrap_plugins: string[];
}

export interface PublishResult {
  ok: boolean;
  warning: boolean;
  message: string;
}

export interface WebhookTestResult {
  delivered: boolean;
  upstream_status: number | null;
  error: string | null;
}

/** Spawnpoint tth filter accepted by the golbat-data markers endpoint. */
export type TthFilter = "All" | "Known" | "Unknown";

/** Optional server-side refinements for the geofence bbox query
 *  (see /v2/geofences ReadQuery). */
export interface NeighborFilters {
  mode?: string;
  projects?: number[];
}

// ---- Import wizard wire shapes (POST /internal/import) --------------------

export type ImportKind = "geofence" | "route";
export type OnCollision = "skip" | "overwrite";
export type ImportAction = "create" | "update" | "skip" | "fail";

export interface ImportItem {
  kind: ImportKind;
  name: string;
  geometry: unknown; // GeoJSON geometry
  mode?: string;
  parent?: string | null;
  projects: number[];
  route_parent?: string | null;
  on_collision: OnCollision;
}

export interface ImportRequest {
  dry_run: boolean;
  items: ImportItem[];
}

export interface ImportOutcome {
  index: number;
  name: string;
  action: ImportAction;
  id: number | null;
  reason: string | null;
}

export interface ImportResult {
  committed: boolean;
  summary: { create: number; update: number; skip: number; fail: number };
  results: ImportOutcome[];
}

/** The full adapter surface. index.live.ts and index.demo.ts must both satisfy it. */
export interface ApiSurface {
  baseDataProvider: DataProvider;
  authProvider: AuthProvider;
  createRealtimeTransport: () => RealtimeTransport;
  loadConfig: () => Promise<ConfigResponse>;
  submitCalc: (body: Record<string, unknown>) => Promise<string>;
  getJob: (id: string) => Promise<JobRecord>;
  getAlgorithms: () => Promise<AlgorithmOptions>;
  // The exact current hook-facing signatures — consumers of these don't change,
  // so both modes must keep implementing precisely what live/* declares.
  fetchMarkers: typeof import("./live/markers").fetchMarkers;
  fetchS2Cells: typeof import("./live/s2").fetchS2Cells;
  fetchFeatureCollection: typeof import("./live/geo-features").fetchFeatureCollection;
  fetchGeofencesByIds: typeof import("./live/geo-features").fetchGeofencesByIds;
  fetchGeofencesByBbox: typeof import("./live/geo-features").fetchGeofencesByBbox;
  postImport: typeof import("./live/import").postImport;
  postConvert: typeof import("./live/import").postConvert;
  publishRecord: (resource: string, id: string | number) => Promise<PublishResult>;
  testWebhook: (id: string | number) => Promise<WebhookTestResult>;
  /** Demo only; no-op in live. */
  resetDemo: () => Promise<void>;
}
