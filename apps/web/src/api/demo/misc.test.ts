import { beforeEach, describe, expect, it } from "vitest";
import { publishGeofences } from "./misc";
import { allRows, deleteRow, putRow } from "./db";
import { resetDemoWorld } from "./seeds/seed";

const dragonite = async (id: number, projectId: number | null, active = true) =>
  putRow("webhooks", {
    id,
    name: `dragonite-${id}`,
    url: "http://dragonite.test",
    secret: null,
    topics: [],
    active,
    project_id: projectId,
    mode: "dragonite",
    method: "POST",
    headers: null,
    created_at: "2026-01-01T00:00:00Z",
    updated_at: "2026-01-01T00:00:00Z",
  });

describe("demo publishGeofences", () => {
  beforeEach(resetDemoWorld);

  it("skips a geofence no Dragonite reaches, a missing id, and writes nothing", async () => {
    const [fence] = await allRows("geofences");
    // The seed's "Demo Dragonite" (id 2) covers every geofence.
    await deleteRow("webhooks", 2);
    const plan = await publishGeofences([fence.id, 999_999], { dryRun: true });

    expect(plan.dry_run).toBe(true);
    expect(plan.writes).toEqual([]);
    expect(plan.skipped.map((s) => [s.geofence, s.reason])).toEqual([
      [fence.id, "no_target"],
      [999_999, "not_found"],
    ]);
  });

  it("names the Dragonite that reaches a geofence but holds no link", async () => {
    const [fence] = await allRows("geofences");
    await dragonite(50, null);
    await dragonite(51, null, false);

    const plan = await publishGeofences([fence.id], { dryRun: false });

    expect(plan.skipped).toHaveLength(1);
    expect(plan.skipped[0].reason).toBe("no_link");
    expect(plan.skipped[0].message).toContain("`dragonite-50` (id 50)");
    expect(plan.skipped[0].message).not.toContain("dragonite-51");
  });
});
