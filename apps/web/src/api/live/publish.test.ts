import { describe, expect, it, vi } from "vitest";

vi.mock("@/lib/http", async (importActual) => ({
  ...(await importActual<typeof import("@/lib/http")>()),
  internalFetch: vi.fn(),
}));

import { internalFetch } from "@/lib/http";
import { publishGeofences } from "./publish";

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
});
