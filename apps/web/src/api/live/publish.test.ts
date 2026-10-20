import { describe, expect, it, vi } from "vitest";

vi.mock("@/lib/http", async (importActual) => ({
  ...(await importActual<typeof import("@/lib/http")>()),
  internalFetch: vi.fn(),
}));

import { internalFetch } from "@/lib/http";
import { publishGeofences } from "./publish";
import { PlanChangedError } from "../shared/plan-changed";

describe("publishGeofences", () => {
  it("POSTs the ids as numbers with the dry_run flag and returns the plan", async () => {
    const plan = { dry_run: true, writes: [], skipped: [] };
    vi.mocked(internalFetch).mockResolvedValueOnce({
      status: 200,
      json: { status: "ok", data: plan },
    });

    const got = await publishGeofences(["3", 4], { dryRun: true });

    expect(got).toEqual(plan);
    expect(internalFetch).toHaveBeenCalledWith("/geofences/publish", {
      method: "POST",
      body: JSON.stringify({ ids: [3, 4], dry_run: true }),
    });
  });

  it("throws when the server refuses", async () => {
    vi.mocked(internalFetch).mockResolvedValueOnce({ status: 500, json: null });
    await expect(publishGeofences([1], { dryRun: false })).rejects.toThrow(/500/);
  });

  it("sends the shown writes and throws PlanChangedError with the plan on a 409", async () => {
    const plan = { dry_run: false, writes: [], skipped: [] };
    vi.mocked(internalFetch).mockResolvedValueOnce({
      status: 409,
      json: { status: "error", error: { code: "plan_changed" }, plan },
    });
    const shown = {
      geofence: 1,
      name: "A",
      mode: "quest",
      webhook_id: 2,
      webhook_name: "d",
      dragonite_area_id: 3,
    };

    const err = await publishGeofences([1], {
      dryRun: false,
      writes: [shown],
    }).catch((e) => e);

    expect(err).toBeInstanceOf(PlanChangedError);
    expect(err.plan).toEqual(plan);
    expect(vi.mocked(internalFetch).mock.calls.at(-1)?.[1]?.body).toBe(
      JSON.stringify({
        ids: [1],
        dry_run: false,
        writes: [{ geofence: 1, webhook_id: 2, dragonite_area_id: 3 }],
      }),
    );
  });
});
