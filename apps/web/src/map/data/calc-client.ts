import { apiV2Fetch, unwrapResponse } from "@/lib/http";

/** A job record as returned by GET /api/v2/jobs/{id} (koji-jobs JobRecord). */
export interface JobRecord {
  id: number | string;
  status: string;
  progress: number;
  phase: string | null;
  result?: { data?: unknown; stats?: unknown } | null;
  error?: string | null;
}

export interface AlgorithmOptions {
  clustering: string[];
  routing: string[];
  bootstrap: string[];
}

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
