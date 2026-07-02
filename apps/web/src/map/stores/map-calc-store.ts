import { create } from "zustand";
import type { CalcMode } from "@/map/lib/calc-request";

export interface CalcJob {
  id: string;
  status: string; // queued | running | succeeded | failed | canceled
  progress: number; // 0..1
  phase: string | null;
}

/** Calc/jobs domain — kept OUT of map-ui-store (separate concern). Panels
 *  subscribe to individual primitives (S3); the result overlay is read by the
 *  DeckCanvas aggregator. */
export interface MapCalcState {
  mode: CalcMode;
  category: string;
  radius: number;
  minPoints: number;
  clusterMode: string | null;
  job: CalcJob | null;
  resultFC: GeoJSON.FeatureCollection | null;
  stats: unknown;
  error: string | null;
  setMode: (m: CalcMode) => void;
  setCategory: (c: string) => void;
  setRadius: (n: number) => void;
  setMinPoints: (n: number) => void;
  setClusterMode: (m: string | null) => void;
  /** Enqueued → track it; clears any prior result/error. */
  startJob: (id: string) => void;
  updateJob: (p: { status: string; progress?: number; phase?: string | null }) => void;
  setResult: (fc: GeoJSON.FeatureCollection | null, stats: unknown) => void;
  setError: (e: string | null) => void;
  /** Clear the job + result overlay (keeps the form settings). */
  clear: () => void;
}

export const useMapCalcStore = create<MapCalcState>()((set) => ({
  mode: "cluster",
  category: "pokestop",
  radius: 70,
  minPoints: 1,
  clusterMode: null,
  job: null,
  resultFC: null,
  stats: null,
  error: null,
  setMode: (mode) => set({ mode }),
  setCategory: (category) => set({ category }),
  setRadius: (radius) => set({ radius }),
  setMinPoints: (minPoints) => set({ minPoints }),
  setClusterMode: (clusterMode) => set({ clusterMode }),
  startJob: (id) =>
    set({ job: { id, status: "queued", progress: 0, phase: null }, resultFC: null, stats: null, error: null }),
  updateJob: (p) =>
    set((s) =>
      s.job
        ? {
            job: {
              ...s.job,
              status: p.status,
              progress: p.progress ?? s.job.progress,
              phase: p.phase ?? s.job.phase,
            },
          }
        : {},
    ),
  setResult: (resultFC, stats) =>
    set((s) => ({ resultFC, stats, job: s.job ? { ...s.job, status: "succeeded", progress: 1 } : null })),
  setError: (error) => set((s) => ({ error, job: s.job ? { ...s.job, status: "failed" } : null })),
  clear: () => set({ job: null, resultFC: null, stats: null, error: null }),
}));
