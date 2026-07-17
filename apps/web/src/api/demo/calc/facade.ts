// The demo calc job facade — the client-only stand-in for the server's
// `POST /api/v2/jobs` + `GET /api/v2/jobs/{id}` + `GET /api/v2/algorithms`.
// It turns a fire-and-forget `submitCalc(body)` into: resolve the data points
// the server would have queried from golbat, run the calc on the wasm worker,
// store a `JobRecord`, and publish `jobs/{id}` status events on the shared
// `demoBus` — which is exactly what `useCalc` subscribes to (no polling loop).
//
// `createCalcFacade(deps)` is a dependency-injection factory so the lifecycle
// is unit-testable with a fake `postCalc` + `resolvePoints` (no real Worker in
// jsdom). The default instance at the bottom wires the real wasm worker and the
// marker store.

import { HttpError } from "@/lib/http";
import type { AlgorithmOptions, JobRecord, TthFilter } from "../../types";
import { getMarkerStore } from "../markers";
import { demoBus, type DemoBus } from "../realtime";

/** A worker (or fake) reply for a calc run. `result` is the wasm `calc()` wire
 *  shape `{ data, stats }`, ready to drop into `JobRecord.result`. */
export type PostCalcReply =
  | { ok: true; result: { data?: unknown; stats?: unknown } }
  | { ok: false; error: string };

/** Runs one calc and resolves its result. Injectable — the default posts to the
 *  wasm worker; tests pass a synchronous fake. */
export type PostCalc = (body: Record<string, unknown>) => Promise<PostCalcReply>;

/** Resolves the `[lat, lon]` data points the server would have queried from the
 *  DB for this request (golbat markers within the area). Injectable. */
export type ResolvePoints = (body: Record<string, unknown>) => Promise<[number, number][]>;

export interface CalcFacadeDeps {
  postCalc: PostCalc;
  resolvePoints: ResolvePoints;
  /** Realtime bus to publish `jobs/{id}` events on. Defaults to the singleton
   *  `demoBus` so the facade and the UI's `useSubscribe` share one instance. */
  bus?: DemoBus;
  /** Resolves the algorithm option lists (default = the wasm worker). */
  getOptions?: () => Promise<AlgorithmOptions>;
}

export interface CalcFacade {
  submitCalc: (body: Record<string, unknown>) => Promise<string>;
  getJob: (id: string) => Promise<JobRecord>;
  getAlgorithms: () => Promise<AlgorithmOptions>;
}

/** Build a calc facade over the injected worker/point-resolution deps. Each
 *  instance keeps its own job store + id counter. */
export function createCalcFacade(deps: CalcFacadeDeps): CalcFacade {
  const bus = deps.bus ?? demoBus;
  const jobs = new Map<string, JobRecord>();
  let counter = 0;
  let algorithmsCache: AlgorithmOptions | null = null;

  function store(id: string, patch: Partial<JobRecord>): JobRecord {
    const rec: JobRecord = {
      id,
      status: "queued",
      progress: 0,
      phase: null,
      result: null,
      error: null,
      ...jobs.get(id),
      ...patch,
    };
    jobs.set(id, rec);
    return rec;
  }

  async function runJob(id: string, body: Record<string, unknown>): Promise<void> {
    try {
      // The server resolves data points from the DB before enqueue; the demo
      // resolves them from the marker store and injects them the same way the
      // wasm `calc` expects (`dataPoints`, camelCase).
      const dataPoints = await deps.resolvePoints(body);
      const enriched = { ...body, dataPoints };
      store(id, { status: "running", progress: 0.1, phase: "clustering" });
      await bus.publish(`jobs/${id}`, {
        type: "status",
        payload: { status: "running", progress: 0.1, phase: "clustering" },
      });
      const reply = await deps.postCalc(enriched);
      if (reply.ok) {
        store(id, { status: "succeeded", progress: 1, phase: "done", result: reply.result, error: null });
        await bus.publish(`jobs/${id}`, {
          type: "status",
          payload: { status: "succeeded", progress: 1, phase: "done" },
        });
      } else {
        store(id, { status: "failed", progress: 1, phase: null, result: null, error: reply.error });
        await bus.publish(`jobs/${id}`, {
          type: "status",
          payload: { status: "failed", error: reply.error },
        });
      }
    } catch (err) {
      const error = err instanceof Error ? err.message : String(err);
      store(id, { status: "failed", progress: 1, phase: null, result: null, error });
      await bus.publish(`jobs/${id}`, { type: "status", payload: { status: "failed", error } });
    }
  }

  async function submitCalc(body: Record<string, unknown>): Promise<string> {
    const id = `demo-${++counter}`;
    // Record the in-flight job up front so a `getJob(id)` immediately after
    // submit returns "running", not a 404.
    store(id, { status: "running", progress: 0.1, phase: "clustering" });
    // Fire-and-forget: return the id promptly and let the calc complete in the
    // background so its realtime events land after the caller has subscribed.
    void runJob(id, body);
    return id;
  }

  async function getJob(id: string): Promise<JobRecord> {
    const rec = jobs.get(id);
    if (!rec) throw new HttpError(404, `job ${id} not found`);
    return rec;
  }

  async function getAlgorithms(): Promise<AlgorithmOptions> {
    if (algorithmsCache) return algorithmsCache;
    const opts = deps.getOptions ? await deps.getOptions() : await postOptions();
    algorithmsCache = opts;
    return opts;
  }

  return { submitCalc, getJob, getAlgorithms };
}

// ── Default instance: real wasm worker + marker store ──────────────────────

interface WorkerReply {
  id: string;
  ok: boolean;
  result?: unknown;
  error?: string;
  ready?: boolean;
  threads?: boolean;
}

let workerRef: Worker | null = null;
const pending = new Map<string, (reply: WorkerReply) => void>();
let msgSeq = 0;

/** Lazily construct the module worker on first use (never at import — a jsdom
 *  unit test importing this module must not spawn a Worker). */
function getWorker(): Worker {
  if (!workerRef) {
    workerRef = new Worker(new URL("./worker.ts", import.meta.url), { type: "module" });
    workerRef.onmessage = (event: MessageEvent<WorkerReply>) => {
      const reply = event.data;
      const resolve = reply.id ? pending.get(reply.id) : undefined;
      if (resolve) {
        pending.delete(reply.id);
        resolve(reply);
      }
    };
    workerRef.onerror = (event: ErrorEvent) => {
      // A worker-level error (e.g. wasm failed to load) can't be routed to a
      // specific request — reject every in-flight call so they don't hang.
      const message = event.message || "calc worker error";
      for (const [id, resolve] of pending) {
        pending.delete(id);
        resolve({ id, ok: false, error: message });
      }
    };
  }
  return workerRef;
}

function callWorker(payload: Record<string, unknown>): Promise<WorkerReply> {
  const id = `w-${++msgSeq}`;
  return new Promise<WorkerReply>((resolve) => {
    pending.set(id, resolve);
    getWorker().postMessage({ ...payload, id });
  });
}

const defaultPostCalc: PostCalc = async (body) => {
  const reply = await callWorker({ kind: "calc", body });
  return reply.ok
    ? { ok: true, result: (reply.result as { data?: unknown; stats?: unknown }) ?? {} }
    : { ok: false, error: reply.error ?? "calc failed" };
};

async function postOptions(): Promise<AlgorithmOptions> {
  const reply = await callWorker({ kind: "options" });
  if (!reply.ok) throw new Error(reply.error ?? "failed to load algorithm options");
  return reply.result as AlgorithmOptions;
}

interface DataFilter {
  lastSeen?: number;
  tth?: TthFilter;
}

/** Server point-resolution parity: cluster/route resolve golbat markers of
 *  `category` within `area`, filtered by `dataFilter.lastSeen/tth`; bootstrap
 *  gets none; reroute/routeStats use the `dataPoints` the caller already sent.
 *  Mirrors `fetchMarkers` exactly (same `lastSeen`/`tth` pass-through), so a
 *  calc clusters the very points the map preview shows. */
export const resolvePointsDefault: ResolvePoints = async (body) => {
  const mode = String(body.mode ?? "");
  if (mode === "route" || mode === "cluster") {
    const category = String(body.category ?? "spawnpoint");
    const area = body.area as GeoJSON.FeatureCollection | undefined;
    const filter = body.dataFilter as DataFilter | undefined;
    const store = await getMarkerStore();
    return store.query(category, {
      areaFeatures: area?.features ?? [],
      lastSeen: filter?.lastSeen,
      tth: filter?.tth,
    });
  }
  // bootstrap → no points; reroute/routeStats → use what the caller sent as-is.
  return (body.dataPoints as [number, number][] | undefined) ?? [];
};

const defaultFacade = createCalcFacade({
  postCalc: defaultPostCalc,
  resolvePoints: resolvePointsDefault,
  getOptions: postOptions,
});

export const submitCalc = defaultFacade.submitCalc;
export const getJob = defaultFacade.getJob;
export const getAlgorithms = defaultFacade.getAlgorithms;
