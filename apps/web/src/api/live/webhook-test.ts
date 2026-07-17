import { internalFetch, unwrapResponse } from "@/lib/http";
import type { WebhookTestResult } from "../types";

/** POST /internal/webhooks/{id}/test — fire a ping delivery and report the
 *  upstream outcome. Throws on non-2xx (via `unwrapResponse`). */
export async function testWebhook(id: string | number): Promise<WebhookTestResult> {
  const res = await internalFetch(`/webhooks/${id}/test`, { method: "POST" });
  return unwrapResponse<WebhookTestResult>(res);
}
