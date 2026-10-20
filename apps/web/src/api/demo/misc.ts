import type { PublishPlan, PublishResult, WebhookTestResult } from "../types";
import { allRows } from "./db";
import { resetDemoWorld } from "./seeds/seed";

/** Publish is a no-op in the demo — there's no upstream Dragonite to sync to,
 *  so it always "succeeds" with a demo marker message. */
export async function publishRecord(
  _resource: string,
  _id: string | number,
): Promise<PublishResult> {
  return { ok: true, warning: false, message: "Published (demo)" };
}

/** The demo has no Dragonite Area links, so nothing is ever written: a
 *  geofence is skipped with the reason the live server would give, depending on
 *  whether an active Dragonite webhook reaches it. */
export async function publishGeofences(
  ids: Array<string | number>,
  _opts: { dryRun: boolean },
): Promise<PublishPlan> {
  const [geofences, hooks] = await Promise.all([
    allRows("geofences"),
    allRows("webhooks"),
  ]);
  const byId = new Map(geofences.map((g) => [g.id, g]));
  const targets = hooks.filter((h) => h.active && h.mode === "dragonite");
  const skipped: PublishPlan["skipped"] = [];
  for (const id of new Set(ids.map(Number))) {
    const fence = byId.get(id);
    if (!fence) {
      skipped.push({
        geofence: id,
        name: null,
        mode: null,
        reason: "not_found",
        message: `no geofence ${id}`,
      });
      continue;
    }
    const reaching = targets.filter(
      (t) => t.project_id == null || fence.projects.includes(t.project_id),
    );
    const base = { geofence: id, name: fence.name, mode: fence.mode };
    skipped.push(
      reaching.length === 0
        ? {
            ...base,
            reason: "no_target",
            message: `no active Dragonite webhook reaches geofence \`${fence.name}\``,
          }
        : {
            ...base,
            reason: "no_link",
            message: `geofence \`${fence.name}\` has no Dragonite area link on the webhooks that reach it: ${reaching
              .map((t) => `\`${t.name}\` (id ${t.id})`)
              .join(", ")}`,
          },
    );
  }
  return { dry_run: _opts.dryRun, writes: [], skipped };
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
