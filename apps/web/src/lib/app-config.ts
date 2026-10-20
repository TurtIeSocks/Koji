import { loadConfig } from "@api";
import type { ConfigResponse } from "@/api/types";

// Module-level cache: every consumer shares one /internal/config fetch, and once
// it resolves new maps mount already configured (no flash on navigation).
let cached: ConfigResponse | null = null;
let inflight: Promise<ConfigResponse | null> | null = null;

/** The config if it has already loaded, else `null`. */
export function getCachedAppConfig(): ConfigResponse | null {
  return cached;
}

/** Fetch `/internal/config` once. Resolves `null` if it fails (unauth / offline). */
export function loadAppConfig(): Promise<ConfigResponse | null> {
  if (cached) return Promise.resolve(cached);
  if (!inflight) {
    inflight = loadConfig()
      .then((c) => (cached = c))
      .catch(() => null);
  }
  return inflight;
}

// Warm the cache at import so the first map after login mounts configured.
void loadAppConfig();
