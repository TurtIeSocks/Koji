import { useCallback, useEffect, useRef, useState } from "react";
import { useSubscribe } from "@/components/realtime";
import { getJob, submitCalc } from "@/map/data/calc-client";
import {
	buildCalcBody,
	type CalcInputs,
	type CalcParams,
	parseCalcResult,
} from "@/map/lib/calc-request";

interface Job {
	id: string;
	status: string;
	progress: number;
	phase: string | null;
}

/** The API serializes JobStatus as PascalCase ("Succeeded"/"Failed"/…); the
 *  realtime topic may use either case. Normalize so comparisons are case-safe. */
const norm = (s: string | undefined): string => (s ?? "").toLowerCase();

const DEFAULTS: CalcParams = {
	mode: "cluster",
	category: "pokestop",
	radius: 70,
	minPoints: 1,
	clusterMode: null,
	sortBy: null,
};

export interface UseCalcReturn {
	params: CalcParams;
	setParams: (p: Partial<CalcParams>) => void;
	job: Job | null;
	result: GeoJSON.FeatureCollection | null;
	stats: unknown;
	error: string | null;
	run: (inputs: CalcInputs) => Promise<void>;
	clear: () => void;
}

/** Headless per-instance calc job state — submit -> track job -> resolve
 *  result. Mirrors `map-calc-store` fields + `useCalcJob` terminal-resolution
 *  behavior, but as local state (no global store) so embedded maps don't
 *  fight over a single calc slot. */
export function useCalc(initial?: Partial<CalcParams>): UseCalcReturn {
	const [params, setParamsState] = useState<CalcParams>({
		...DEFAULTS,
		...initial,
	});
	const [job, setJob] = useState<Job | null>(null);
	const [result, setResult] = useState<GeoJSON.FeatureCollection | null>(null);
	const [stats, setStats] = useState<unknown>(null);
	const [error, setError] = useState<string | null>(null);
	const jobId = job?.id ?? null;
	const resolvedRef = useRef<string | null>(null);

	const setParams = useCallback(
		(p: Partial<CalcParams>) => setParamsState((s) => ({ ...s, ...p })),
		[],
	);

	const resolveTerminal = useCallback(async (id: string) => {
		if (resolvedRef.current === id) return;
		try {
			const rec = await getJob(id);
			const st = norm(rec.status);
			if (st === "succeeded") {
				// Latch ONLY once terminal — the safety-net effect fires this while
				// the job is still queued/running; latching before the status check
				// would block the later realtime succeeded/failed event from ever
				// resolving the result (silent data loss). Double-resolution on a
				// terminal job is idempotent, so latching-on-terminal is enough.
				resolvedRef.current = id;
				const r = parseCalcResult(rec);
				setResult(r.fc);
				setStats(r.stats);
				setJob((j) => j && { ...j, status: "succeeded", progress: 1 });
			} else if (st === "failed") {
				resolvedRef.current = id;
				setError(rec.error ?? "job failed");
				setJob((j) => j && { ...j, status: "failed" });
			}
		} catch (e) {
			setError(e instanceof Error ? e.message : "failed to fetch job result");
		}
	}, []);

	useSubscribe<{ status?: string; progress?: number; phase?: string | null }>(
		jobId ? `jobs/${jobId}` : "",
		(event) => {
			const p = event.payload;
			const st = norm(p?.status);
			if (!st || !jobId) return;
			setJob(
				(j) =>
					j && {
						...j,
						status: st,
						progress: p?.progress ?? j.progress,
						phase: p?.phase ?? j.phase,
					},
			);
			if (st === "succeeded" || st === "failed") void resolveTerminal(jobId);
		},
		{ enabled: jobId != null },
	);

	// Safety net: fetch once when a job appears (finished-before-subscribe / reload).
	useEffect(() => {
		if (jobId) void resolveTerminal(jobId);
	}, [jobId, resolveTerminal]);

	const run = useCallback(
		async (inputs: CalcInputs) => {
			setError(null);
			setResult(null);
			setStats(null);
			resolvedRef.current = null;
			try {
				const id = await submitCalc(buildCalcBody(params, inputs));
				setJob({ id, status: "queued", progress: 0, phase: null });
			} catch (e) {
				setError(e instanceof Error ? e.message : "failed to submit calc");
			}
		},
		[params],
	);

	const clear = useCallback(() => {
		setJob(null);
		setResult(null);
		setStats(null);
		setError(null);
		resolvedRef.current = null;
	}, []);

	return { params, setParams, job, result, stats, error, run, clear };
}
