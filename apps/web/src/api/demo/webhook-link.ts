import { allRows } from "./db";
import type { LinkDragonitePlan } from "../types";

/** The demo has no Dragonite to read Areas from, so it makes up a plan from the
 *  demo geofences, by position: a share linked, a share already linked, a few
 *  conflicts, the rest unmatched, and a few Areas nothing is named after.
 *  Applying writes nothing; it answers the same plan as a real run. */
export async function linkDragonite(
  webhookId: string | number,
  { dryRun }: { dryRun: boolean },
): Promise<LinkDragonitePlan> {
  const geofences = (await allRows("geofences")).sort((a, b) => a.id - b.id);
  const plan: LinkDragonitePlan = {
    webhook_id: Number(webhookId),
    dry_run: dryRun,
    linked: [],
    already_linked: [],
    conflicts: [],
    area_taken: [],
    areas_without_geofence: [],
    geofences_without_area: [],
  };
  geofences.forEach((g, i) => {
    const area = 1000 + i;
    const { id: geofence, name, mode } = g;
    switch (i % 5) {
      case 0:
      case 1:
        plan.linked.push({ geofence, name, mode, dragonite_area_id: area });
        break;
      case 2:
        plan.already_linked.push({ geofence, name, mode, dragonite_area_id: area });
        break;
      case 3:
        plan.conflicts.push({
          geofence,
          name,
          mode,
          dragonite_area_id: area + 5000,
          wanted_dragonite_area_id: area,
        });
        break;
      default:
        plan.geofences_without_area.push({ geofence, name, mode });
    }
  });
  for (const [i, name] of ["Demo Area A", "Demo Area B", "Demo Area C"].entries()) {
    plan.areas_without_geofence.push({ dragonite_area_id: 9000 + i, name });
  }
  return plan;
}
