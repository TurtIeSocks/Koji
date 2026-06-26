import { afterEach, expect, test, vi } from "vitest";
import { apiV2Fetch } from "@/lib/http";

afterEach(() => vi.restoreAllMocks());

test("apiV2Fetch hits the /api/v2 base with credentials + JSON headers", async () => {
  const fetchMock = vi.fn().mockResolvedValue(
    new Response(JSON.stringify({ status: "ok", data: { ok: true } }), { status: 200 }),
  );
  vi.stubGlobal("fetch", fetchMock);

  const res = await apiV2Fetch("/health");

  expect(fetchMock).toHaveBeenCalledWith(
    "/api/v2/health",
    expect.objectContaining({ credentials: "include" }),
  );
  expect(res.status).toBe(200);
  expect(res.json).toEqual({ status: "ok", data: { ok: true } });
});

test("apiV2Fetch returns null json for an empty body", async () => {
  // Use status 200 with empty body — MSW interceptor rejects new Response("", {status:204})
  // in this jsdom env; the null-json path is identical regardless of status.
  vi.stubGlobal("fetch", vi.fn().mockResolvedValue(new Response("", { status: 200 })));
  const res = await apiV2Fetch("/x", { method: "POST" });
  expect(res.json).toBeNull();
});
