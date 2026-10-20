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

/** One geofence going to one Dragonite Area (POST /geofences/publish). */
export interface PublishWrite {
  geofence: number;
  name: string;
  mode: string;
  webhook_id: number;
  webhook_name: string;
  dragonite_area_id: number;
  /** Set once the write is queued; absent on a dry run. */
  event_id?: string;
}

export type PublishSkipReason = "not_found" | "no_target" | "no_link";

/** A geofence a publish sends nothing for. */
export interface PublishSkip {
  geofence: number;
  /** Null for an id with no geofence. */
  name: string | null;
  mode: string | null;
  reason: PublishSkipReason;
  message: string;
}

/** What POST /geofences/publish writes (or, on a dry run, would write). */
export interface PublishPlan {
  dry_run: boolean;
  writes: PublishWrite[];
  skipped: PublishSkip[];
}

export interface WebhookTestResult {
  delivered: boolean;
  upstream_status: number | null;
  /** Dragonite rows only: how many Areas the Dragonite answered with. */
  area_count?: number | null;
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
  /** Dry run: the writes and skipped geofences for `ids`, nothing sent.
   *  Otherwise sends those writes and answers them with their event ids. */
  publishGeofences: (
    ids: Array<string | number>,
    opts: { dryRun: boolean },
  ) => Promise<PublishPlan>;
  testWebhook: (id: string | number) => Promise<WebhookTestResult>;
  /** Demo only; no-op in live. */
  resetDemo: () => Promise<void>;
}
