import { apiV2Fetch, unwrapResponse } from "@/lib/http";
import type { AlgorithmOptions, JobRecord } from "../types";

/** POST /api/v2/jobs → the new job's id (always async, 202). */
export async function submitCalc(body: Record<string, unknown>): Promise<string> {
  const res = await apiV2Fetch("/jobs", { method: "POST", body: JSON.stringify(body) });
  const data = unwrapResponse<{ job_id: number | string }>(res);
  return String(data.job_id);
}

/** GET /api/v2/jobs/{id} — the current job record (result present once succeeded). */
export async function getJob(id: string): Promise<JobRecord> {
  return unwrapResponse<JobRecord>(await apiV2Fetch(`/jobs/${id}`));
}

/** GET /api/v2/algorithms — available clustering / routing / bootstrap modes. */
export async function getAlgorithms(): Promise<AlgorithmOptions> {
  return unwrapResponse<AlgorithmOptions>(await apiV2Fetch("/algorithms"));
}
