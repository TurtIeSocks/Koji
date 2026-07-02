import { useEffect } from "react";
import { useSubscribe } from "@/components/realtime";
import { useMapCalcStore } from "@/map/stores/map-calc-store";
import { getJob } from "@/map/data/calc-client";
import { parseCalcResult } from "@/map/lib/calc-request";

interface JobEventPayload {
  id?: string | number;
  status?: string;
  progress?: number;
  phase?: string | null;
}

type SetResult = (fc: GeoJSON.FeatureCollection | null, stats: unknown) => void;
type SetError = (e: string | null) => void;

/** Fetch the job record and push its terminal outcome into the store. */
async function resolveTerminal(id: string, setResult: SetResult, setError: SetError): Promise<void> {
  try {
    const rec = await getJob(id);
    if (rec.status === "succeeded") {
      const { fc, stats } = parseCalcResult(rec);
      setResult(fc, stats);
    } else if (rec.status === "failed") {
      setError(rec.error ?? "job failed");
    }
  } catch (e) {
    setError(e instanceof Error ? e.message : "failed to fetch job result");
  }
}

/** Watches the active calc job: live progress via the `jobs/{id}` realtime topic,
 *  then fetches the result on success (or surfaces the error). No-op when idle. */
export function useCalcJob(): void {
  const jobId = useMapCalcStore((s) => s.job?.id ?? null);
  const updateJob = useMapCalcStore((s) => s.updateJob);
  const setResult = useMapCalcStore((s) => s.setResult);
  const setError = useMapCalcStore((s) => s.setError);

  // Live progress + terminal notifications.
  useSubscribe<JobEventPayload>(
    jobId ? `jobs/${jobId}` : "",
    (event) => {
      const p = event.payload;
      if (!p?.status || !jobId) return;
      updateJob({ status: p.status, progress: p.progress, phase: p.phase ?? undefined });
      if (p.status === "succeeded" || p.status === "failed") {
        void resolveTerminal(jobId, setResult, setError);
      }
    },
    { enabled: jobId != null },
  );

  // Safety net: one immediate fetch when a job appears, so a job that already
  // finished before we subscribed (fast job, or a reload mid-run) still resolves.
  useEffect(() => {
    if (!jobId) return;
    let cancelled = false;
    void (async () => {
      try {
        const rec = await getJob(jobId);
        if (cancelled) return;
        updateJob({ status: rec.status, progress: rec.progress, phase: rec.phase ?? undefined });
        if (rec.status === "succeeded") {
          const { fc, stats } = parseCalcResult(rec);
          setResult(fc, stats);
        } else if (rec.status === "failed") {
          setError(rec.error ?? "job failed");
        }
      } catch {
        /* transient — the realtime subscription will still drive it */
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [jobId, updateJob, setResult, setError]);
}
