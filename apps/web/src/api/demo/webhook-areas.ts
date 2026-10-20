import type { WebhookArea } from "../types";
import { allRows } from "./db";

/** The Areas the demo's stand-in Dragonite lists. There is no upstream in the
 *  serverless demo, so every Dragonite target answers with these, in the
 *  unsorted order a real Dragonite may use. */
const DEMO_AREAS = [
  { id: 14, name: "Williamsburg" },
  { id: 11, name: "midtown" },
  { id: 12, name: "Central Park" },
  { id: 15, name: "Staten Island" },
  { id: 13, name: "Chelsea" },
  { id: 16, name: "Harlem" },
];

/** The demo's `GET /webhooks/{id}/areas`: the canned Areas, each with the
 *  stored geofences linked to it on that target (as the live endpoint does). */
export async function listWebhookAreas(id: string | number): Promise<WebhookArea[]> {
  const target = (await allRows("webhooks")).find((w) => String(w.id) === String(id));
  if (!target) throw new Error(`no webhook ${id}`);
  if (target.mode !== "dragonite") throw new Error(`webhook ${id} is not a Dragonite`);
  const geofences = await allRows("geofences");
  return DEMO_AREAS.map((area) => ({
    ...area,
    held_by: geofences
      .filter((g) =>
        (g.dragonite_links ?? []).some(
          (l) => l.webhook_id === target.id && l.dragonite_area_id === area.id,
        ),
      )
      .map((g) => ({ geofence: g.id, name: g.name, mode: g.mode })),
  }));
}
