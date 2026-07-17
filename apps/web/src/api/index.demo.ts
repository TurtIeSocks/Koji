// The DEMO `@api` implementation — a client-only stand-in for the live
// backend, used by the static GitHub Pages demo build (`vite --mode demo`;
// see vite.config.ts). Must satisfy `ApiSurface` (see api-surface.test.ts),
// same as index.live.ts.
//
// Task 7 fills in the data/geo/import/misc surfaces (idb CRUD + synthetic
// world + wasm-backed geometry). The realtime transport and the calc trio
// (`submitCalc`/`getJob`/`getAlgorithms`) stay stubbed — they land in Task 8
// alongside the in-memory realtime bus and the wasm calc worker
// (`apps/web/src/api/demo/realtime.ts` + `demo/calc/*`, per the plan). Those
// stubs throw only when *called* (a user running a calc), so the demo app
// still boots and browses; nothing invokes them at module load.
import type { RealtimeTransport } from "@/components/realtime";
import type { AlgorithmOptions, JobRecord } from "./types";
// Not part of ApiSurface, but src/data-provider.ts imports it directly from
// "@api" (a historical re-export path) — the demo bundle needs a real export
// here or the build fails with a bundler MISSING_EXPORT error.
export { serializeGeofenceWrite } from "./shared/serialize-geofence-write";

const NOT_IMPLEMENTED = "demo: not implemented yet (Task 8)";

/** Shared stub body for the not-yet-implemented calc trio. Zero params + an
 *  inferred `never` return satisfies every differently-shaped member. */
const stub = (): never => {
  throw new Error(NOT_IMPLEMENTED);
};

export { demoBaseDataProvider as baseDataProvider } from "./demo/data-provider";
export { authProvider } from "./demo/auth-provider";
export { loadConfig } from "./demo/config";
export { fetchMarkers } from "./demo/markers";
export { fetchS2Cells } from "./demo/s2";
export {
  fetchFeatureCollection,
  fetchGeofencesByBbox,
  fetchGeofencesByIds,
} from "./demo/geo-features";
export { postConvert, postImport } from "./demo/import";
export { publishRecord, resetDemo, testWebhook } from "./demo/misc";

// Re-declared so the exported symbol keeps its precise `ApiSurface` shape even
// though it's a stub for now. `DataProvider`/`AuthProvider` are structurally
// satisfied by the demo implementations above; these are the Task-8 holdouts.
export const createRealtimeTransport = (): RealtimeTransport => stub();
export const submitCalc = (): Promise<string> => stub();
export const getJob = (): Promise<JobRecord> => stub();
export const getAlgorithms = (): Promise<AlgorithmOptions> => stub();
