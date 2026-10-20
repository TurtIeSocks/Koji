import { internalFetch, unwrapResponse } from "@/lib/http";
import type { LinkDragonitePlan } from "../types";

/** POST /internal/geofences/link-dragonite — link a Dragonite target's
 *  geofences to its Areas by name. A dry run answers the plan and writes
 *  nothing; otherwise `dry_run` is left out of the body. Throws on non-2xx
 *  with the server's message when it sent one. */
export async function linkDragonite(
  webhookId: string | number,
  { dryRun }: { dryRun: boolean },
): Promise<LinkDragonitePlan> {
  const res = await internalFetch("/geofences/link-dragonite", {
    method: "POST",
    body: JSON.stringify({
      webhook_id: Number(webhookId),
      ...(dryRun ? { dry_run: true } : {}),
    }),
  });
  if (res.status < 200 || res.status >= 300) {
    const error = (res.json as { error?: string | { message?: string } } | null)?.error;
    const message = typeof error === "string" ? error : error?.message;
    throw new Error(message ?? `Link by name failed (${res.status})`);
  }
  return unwrapResponse<LinkDragonitePlan>(res);
}
