// The LIVE `@api` implementation — every network surface of the app behind one
// module. The `@api` alias (vite.config.ts + tsconfig.app.json + vitest.config.ts)
// resolves here today; a demo implementation (index.demo.ts) must satisfy the
// same `ApiSurface` (see api-surface.test.ts).
export type * from "./types";

export { authProvider } from "./live/auth-provider";
export { baseDataProvider, serializeGeofenceWrite } from "./live/data-provider";
export { createRealtimeTransport } from "./live/realtime";
export { loadConfig, resetDemo } from "./live/config";
export { getAlgorithms, getJob, submitCalc } from "./live/calc";
export { fetchMarkers } from "./live/markers";
export { fetchS2Cells } from "./live/s2";
export {
  fetchFeatureCollection,
  fetchGeofencesByBbox,
  fetchGeofencesByIds,
} from "./live/geo-features";
export { postConvert, postImport } from "./live/import";
export { publishRecord } from "./live/publish";
export { testWebhook } from "./live/webhook-test";
