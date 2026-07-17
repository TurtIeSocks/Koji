import { internalFetch } from "@/lib/http";
import { segFor } from "./data-provider";
import type { PublishResult } from "../types";

/**
 * POST /internal/{seg}/{id}/publish for one record.
 * - 2xx → `{ ok: true }`
 * - 422 → `{ ok: false, warning: true }` (no linked Dragonite area); the
 *   message is the server's `error` string when present.
 * - other → `{ ok: false, warning: false }`
 * Network failures still throw (callers keep their catch).
 */
export async function publishRecord(
  resource: string,
  id: string | number,
): Promise<PublishResult> {
  const res = await internalFetch(`/${segFor(resource)}/${id}/publish`, {
    method: "POST",
  });
  if (res.status === 422) {
    const body = res.json as { error?: string } | null;
    return {
      ok: false,
      warning: true,
      message: body?.error ?? "Cannot publish: no linked Dragonite area",
    };
  }
  if (res.status < 200 || res.status >= 300) {
    return { ok: false, warning: false, message: `Publish failed (${res.status})` };
  }
  return { ok: true, warning: false, message: "Published" };
}
