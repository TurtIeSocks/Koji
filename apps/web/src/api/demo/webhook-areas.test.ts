import { beforeAll, describe, expect, it } from "vitest";
import { demoBaseDataProvider } from "./data-provider";
import { ensureSeeded } from "./seeds/seed";
import { listWebhookAreas } from "./webhook-areas";

beforeAll(async () => {
  await ensureSeeded();
});

const DRAGONITE = 2;

describe("demo Dragonite Areas", () => {
  it("lists the stand-in Dragonite's Areas and refuses a webhook that is not a Dragonite", async () => {
    const areas = await listWebhookAreas(DRAGONITE);
    expect(areas.map((a) => a.name)).toContain("midtown");
    expect(areas.every((a) => a.held_by.length === 0)).toBe(true);
    await expect(listWebhookAreas(1)).rejects.toThrow("not a Dragonite");
  });

  it("stores a geofence's link, reports its holder, and drops the link on null", async () => {
    const midtown = (await listWebhookAreas(DRAGONITE)).find((a) => a.name === "midtown")!;
    const fence = (
      await demoBaseDataProvider.getList("geofence", {
        pagination: { page: 1, perPage: 1 },
        sort: { field: "id", order: "ASC" },
        filter: {},
      })
    ).data[0];

    await demoBaseDataProvider.update("geofence", {
      id: fence.id,
      previousData: {},
      data: { dragonite_links: [{ webhook_id: DRAGONITE, dragonite_area_id: midtown.id }] },
    });
    const held = (await listWebhookAreas(DRAGONITE)).find((a) => a.id === midtown.id)!;
    const record = (await demoBaseDataProvider.getOne("geofence", { id: fence.id })).data;

    expect(held.held_by).toEqual([{ geofence: fence.id, name: fence.name, mode: fence.mode }]);
    expect(record.dragonite_links).toEqual([
      { webhook_id: DRAGONITE, dragonite_area_id: midtown.id },
    ]);

    await demoBaseDataProvider.update("geofence", {
      id: fence.id,
      previousData: {},
      data: { dragonite_links: [{ webhook_id: DRAGONITE, dragonite_area_id: null }] },
    });
    const freed = (await listWebhookAreas(DRAGONITE)).find((a) => a.id === midtown.id)!;
    expect(freed.held_by).toEqual([]);
  });
});
