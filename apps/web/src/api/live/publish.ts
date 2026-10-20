import { internalFetch, unwrapResponse } from "@/lib/http";
import { segFor } from "./data-provider";
import type { PublishPlan, PublishResult, PublishWrite } from "../types";
import { PlanChangedError } from "../shared/plan-changed";

/**
 * POST /internal/{seg}/{id}/publish for one record.
 * - 2xx → `{ ok: true }`
 * - 422 → `{ ok: false, warning: true }` (no Dragonite webhook reaches the
 *   record, or none that does links it); the message is the server's error
 *   message, which says which, when present.
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
    const body = res.json as { error?: string | { message?: string } } | null;
    const error = body?.error;
    return {
      ok: false,
      warning: true,
      message:
        (typeof error === "string" ? error : error?.message) ??
        "Cannot publish: no linked Dragonite area",
    };
  }
  if (res.status < 200 || res.status >= 300) {
    return { ok: false, warning: false, message: `Publish failed (${res.status})` };
  }
  return { ok: true, warning: false, message: "Published" };
}

/**
 * POST /internal/geofences/publish for `ids`. With `dryRun` the server answers
 * the writes and skipped geofences and sends nothing; without it, it sends
 * the same plan. Give the `writes` that were shown and a send only goes out if
 * the server's plan still equals them: otherwise nothing is sent and this
 * throws `PlanChangedError` with the current plan. Throws on any other non-2xx.
 */
export async function publishGeofences(
  ids: Array<string | number>,
  opts: { dryRun: boolean; writes?: PublishWrite[] },
): Promise<PublishPlan> {
  const res = await internalFetch("/geofences/publish", {
    method: "POST",
    body: JSON.stringify({
      ids: ids.map(Number),
      dry_run: opts.dryRun,
      ...(opts.writes && {
        writes: opts.writes.map((w) => ({
          geofence: w.geofence,
          webhook_id: w.webhook_id,
          dragonite_area_id: w.dragonite_area_id,
        })),
      }),
    }),
  });
  if (res.status === 409) {
    const plan = (res.json as { plan?: PublishPlan } | null)?.plan;
    if (plan) throw new PlanChangedError(plan);
  }
  return unwrapResponse<PublishPlan>(res);
}
