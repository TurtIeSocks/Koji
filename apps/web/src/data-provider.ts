import { baseDataProvider, createRealtimeTransport } from "@api";
import {
  inMemoryLockProvider,
  realtimeDataProvider,
} from "@/components/realtime";

// Consumers historically import these from here — preserve the path.
export { baseDataProvider, serializeGeofenceWrite } from "@api";

/** The app-wide provider: the mode-agnostic realtime composition over
 *  whichever base `@api` resolves to (live today, demo later). */
export const dataProvider = realtimeDataProvider(
  baseDataProvider,
  createRealtimeTransport(),
  { locks: inMemoryLockProvider() },
);
