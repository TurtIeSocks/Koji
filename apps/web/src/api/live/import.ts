import { internalFetch, unwrapResponse } from "@/lib/http";
import type { ImportRequest, ImportResult } from "../types";

/** POST a bulk import (dry-run preview or real commit) to the atomic
 *  `/internal/import` endpoint and unwrap the `{status,data}` envelope. */
export async function postImport(body: ImportRequest): Promise<ImportResult> {
  const res = await internalFetch("/import", {
    method: "POST",
    body: JSON.stringify(body),
  });
  return unwrapResponse<ImportResult>(res);
}

/** Normalize a FeatureCollection server-side via /internal/geometry/convert.
 *  Returns the normalized features. */
export async function postConvert(features: unknown[]): Promise<unknown[]> {
  const res = await internalFetch("/geometry/convert", {
    method: "POST",
    // ConvertReq = { area, output (serde default), simplify? }. `output` is
    // optional — omitted, convert returns a FeatureCollection (default return
    // type derived from the FeatureCollection `area`). Verified vs
    // crates/koji-service/src/requests/ops.rs:255 ConvertReq.
    body: JSON.stringify({ area: { type: "FeatureCollection", features } }),
  });
  const data = unwrapResponse<{ features?: unknown[] } | unknown[]>(res);
  // convert returns a FeatureCollection (in the envelope) — pull its features.
  if (Array.isArray(data)) return data;
  return (data as { features?: unknown[] }).features ?? [];
}
