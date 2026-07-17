import { afterEach, expect, test, vi } from "vitest";
import { submitCalc, getJob } from "./calc";

const { apiV2FetchMock } = vi.hoisted(() => ({ apiV2FetchMock: vi.fn() }));
vi.mock("@/lib/http", async (importActual) => {
  const actual = await importActual<typeof import("@/lib/http")>();
  return { ...actual, apiV2Fetch: apiV2FetchMock };
});

afterEach(() => apiV2FetchMock.mockReset());

test("submitCalc POSTs the body and returns the job id as a string", async () => {
  apiV2FetchMock.mockResolvedValue({ status: 202, json: { status: "ok", data: { job_id: 7 } } });
  const id = await submitCalc({ mode: "cluster", category: "gym" });
  expect(id).toBe("7");
  expect(apiV2FetchMock).toHaveBeenCalledWith("/jobs", expect.objectContaining({ method: "POST" }));
  expect(JSON.parse((apiV2FetchMock.mock.calls[0][1] as RequestInit).body as string)).toMatchObject({ mode: "cluster" });
});

test("getJob unwraps the job record", async () => {
  const record = { id: 7, status: "succeeded", progress: 1, phase: "done", result: { data: {}, stats: {} } };
  apiV2FetchMock.mockResolvedValue({ status: 200, json: { status: "ok", data: record } });
  expect(await getJob("7")).toEqual(record);
});

test("submitCalc surfaces a non-2xx as an HttpError", async () => {
  apiV2FetchMock.mockResolvedValue({ status: 400, json: { status: "error", error: "bad" } });
  await expect(submitCalc({})).rejects.toThrow();
});
