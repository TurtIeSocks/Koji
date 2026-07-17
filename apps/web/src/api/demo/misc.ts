import type { PublishResult, WebhookTestResult } from "../types";
import { resetDemoWorld } from "./seeds/seed";

/** Publish is a no-op in the demo — there's no upstream Dragonite to sync to,
 *  so it always "succeeds" with a demo marker message. */
export async function publishRecord(
  _resource: string,
  _id: string | number,
): Promise<PublishResult> {
  return { ok: true, warning: false, message: "Published (demo)" };
}

/** Webhook delivery is disabled in the serverless demo — report a non-delivery
 *  rather than pretending a ping went out. */
export async function testWebhook(_id: string | number): Promise<WebhookTestResult> {
  return {
    delivered: false,
    upstream_status: null,
    error: "demo mode — outbound delivery disabled",
  };
}

/** Wipe + reseed the demo world. Pure (no reload) — the settings/debug UI owns
 *  the `location.reload()` that follows, so this stays testable. */
export async function resetDemo(): Promise<void> {
  await resetDemoWorld();
}
