import { internalFetch, unwrapResponse } from "@/lib/http";
import type { ConfigResponse } from "../types";

/** GET /internal/config — server runtime config (start center, tile server,
 *  plugin lists, …). Throws on non-2xx (unauth / offline); callers decide the
 *  fallback. */
export async function loadConfig(): Promise<ConfigResponse> {
  return unwrapResponse<ConfigResponse>(await internalFetch("/config"));
}

/** Demo only; no-op in live. */
export const resetDemo = async (): Promise<void> => {};
