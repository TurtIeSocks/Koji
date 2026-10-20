import { internalFetch, unwrapResponse } from "@/lib/http";
import type { WebhookArea } from "../types";

/** GET /internal/webhooks/{id}/areas — a Dragonite target's Areas, with who
 *  holds each. Throws on non-2xx (via `unwrapResponse`). */
export async function listWebhookAreas(id: string | number): Promise<WebhookArea[]> {
  const res = await internalFetch(`/webhooks/${id}/areas`);
  return unwrapResponse<WebhookArea[]>(res);
}
