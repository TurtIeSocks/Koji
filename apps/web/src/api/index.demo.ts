// The DEMO `@api` implementation — a client-only stand-in for the live
// backend, used by the static GitHub Pages demo build (`vite --mode demo`;
// see vite.config.ts). Must satisfy `ApiSurface` (see api-surface.test.ts),
// same as index.live.ts.
//
// Task 7 filled in the data/geo/import/misc surfaces (idb CRUD + synthetic
// world + wasm-backed geometry). Task 8 adds the realtime transport (an
// in-memory `demoBus`) and the calc trio (`submitCalc`/`getJob`/
// `getAlgorithms`, backed by the wasm worker + job facade). The COI service
// worker + world/route seeding are driven from `main.tsx`'s demo boot.
// Not part of ApiSurface, but src/data-provider.ts imports it directly from
// "@api" (a historical re-export path) — the demo bundle needs a real export
// here or the build fails with a bundler MISSING_EXPORT error.
export { serializeGeofenceWrite } from "./shared/serialize-geofence-write";

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

// The in-memory realtime bus + the wasm-worker-backed calc facade. `useCalc`
// subscribes to the bus's `jobs/{id}` topic and the facade publishes to that
// SAME singleton bus — the whole no-polling calc loop.
export { createRealtimeTransport } from "./demo/realtime";
export { getAlgorithms, getJob, submitCalc } from "./demo/calc/facade";
