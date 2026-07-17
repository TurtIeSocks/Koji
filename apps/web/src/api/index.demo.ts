// The DEMO `@api` implementation — a client-only stand-in for the live
// backend, used by the static GitHub Pages demo build (`vite --mode demo`;
// see vite.config.ts). Every surface throws until a later task fills it in
// with real in-memory/seed-backed behavior; `resetDemo` is the sole
// exception (a demo-only lifecycle hook — no-op until there's state to
// reset). Must satisfy `ApiSurface` (see api-surface.test.ts), same as
// index.live.ts.
import type { AuthProvider, DataProvider } from "ra-core";
import type { RealtimeTransport } from "@/components/realtime";
import type {
  AlgorithmOptions,
  ConfigResponse,
  JobRecord,
  PublishResult,
  WebhookTestResult,
} from "./types";
// Not part of ApiSurface, but src/data-provider.ts imports it directly from
// "@api" (a historical re-export path) — the demo bundle needs a real export
// here or the build fails with a bundler MISSING_EXPORT error. Pure/mode-
// agnostic, so reuse the same implementation live uses (see the module).
export { serializeGeofenceWrite } from "./shared/serialize-geofence-write";

const NOT_IMPLEMENTED = "demo: not implemented yet";

/** Shared stub body for every not-yet-implemented surface. Declared with zero
 *  parameters and an inferred `never` return: TypeScript allows a function
 *  with fewer parameters than the target signature (the extras are simply
 *  never passed), and `never` is assignable to any declared return type
 *  (including every `Promise<...>` in `ApiSurface`) — so this one stub
 *  satisfies every differently-shaped member below. */
const stub = (): never => {
  throw new Error(NOT_IMPLEMENTED);
};

export const baseDataProvider: DataProvider = {
  getList: stub,
  getOne: stub,
  getMany: stub,
  getManyReference: stub,
  create: stub,
  update: stub,
  updateMany: stub,
  delete: stub,
  deleteMany: stub,
};

export const authProvider: AuthProvider = {
  login: stub,
  logout: stub,
  checkAuth: stub,
  checkError: stub,
  getPermissions: stub,
  canAccess: stub,
};

export const createRealtimeTransport = (): RealtimeTransport => stub();

export const loadConfig = (): Promise<ConfigResponse> => stub();
export const submitCalc = (): Promise<string> => stub();
export const getJob = (): Promise<JobRecord> => stub();
export const getAlgorithms = (): Promise<AlgorithmOptions> => stub();

export const fetchMarkers: typeof import("./live/markers").fetchMarkers = stub;
export const fetchS2Cells: typeof import("./live/s2").fetchS2Cells = stub;
export const fetchFeatureCollection: typeof import("./live/geo-features").fetchFeatureCollection =
  stub;
export const fetchGeofencesByIds: typeof import("./live/geo-features").fetchGeofencesByIds = stub;
export const fetchGeofencesByBbox: typeof import("./live/geo-features").fetchGeofencesByBbox =
  stub;

export const postImport: typeof import("./live/import").postImport = stub;
export const postConvert: typeof import("./live/import").postConvert = stub;

export const publishRecord = (): Promise<PublishResult> => stub();
export const testWebhook = (): Promise<WebhookTestResult> => stub();

/** Demo lifecycle hook: no-op until there's demo state (seeded fixtures, an
 *  in-memory store, ...) worth resetting. */
export const resetDemo = async (): Promise<void> => {};
