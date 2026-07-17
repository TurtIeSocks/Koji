// The demo calc Web Worker (module worker). It owns its OWN `@koji-wasm`
// instance — a worker can't share the main thread's wasm singleton
// (`api/demo/wasm.ts`), so it initialises the module and (where cross-origin
// isolation gives us SharedArrayBuffer) spins up the rayon thread pool here.
// Running the calc off the main thread keeps a heavy cluster/route from
// freezing the map UI.
//
// Message protocol (every request carries a correlation `id`, every reply
// echoes it):
//   { kind: "ping",    id }              -> { id, ok: true, ready: true, threads }
//   { kind: "options", id }              -> { id, ok: true, result: <algorithm options> }
//   { kind: "calc",    id, body }        -> { id, ok: true, result: { data, stats } }
//                                        |  { id, ok: false, error }
//
// `calc(body)` is the wasm mirror of `POST /api/v2/jobs`; `body` is the exact
// jobs wire body with `dataPoints`/`clusters` already injected by the facade
// (the server would resolve those from the DB — the worker never touches a DB).

import initWasm, { algorithm_options, calc, initThreadPool } from "@koji-wasm";

interface CalcMessage {
  kind: "calc";
  id: string;
  body: unknown;
}
interface OptionsMessage {
  kind: "options";
  id: string;
}
interface PingMessage {
  kind: "ping";
  id: string;
}
type WorkerRequest = CalcMessage | OptionsMessage | PingMessage;

let readyPromise: Promise<{ threads: boolean }> | null = null;

/** Instantiate the wasm module once, then (best-effort) the rayon thread pool.
 *  The pool needs SharedArrayBuffer, which only exists under cross-origin
 *  isolation — a non-isolated page still gets a working single-threaded
 *  module rather than a throw. */
function ensureReady(): Promise<{ threads: boolean }> {
  if (!readyPromise) {
    readyPromise = (async () => {
      await initWasm();
      let threads = false;
      if (typeof SharedArrayBuffer !== "undefined") {
        try {
          const n =
            typeof navigator !== "undefined" && navigator.hardwareConcurrency
              ? navigator.hardwareConcurrency
              : 4;
          await initThreadPool(n);
          threads = true;
        } catch {
          // Thread-pool init is best-effort; fall back to single-threaded.
        }
      }
      return { threads };
    })();
  }
  return readyPromise;
}

self.onmessage = async (event: MessageEvent<WorkerRequest>) => {
  const msg = event.data;
  try {
    if (msg.kind === "ping") {
      const { threads } = await ensureReady();
      self.postMessage({ id: msg.id, ok: true, ready: true, threads });
      return;
    }
    if (msg.kind === "options") {
      await ensureReady();
      self.postMessage({ id: msg.id, ok: true, result: algorithm_options() });
      return;
    }
    if (msg.kind === "calc") {
      await ensureReady();
      const result = calc(msg.body);
      self.postMessage({ id: msg.id, ok: true, result });
      return;
    }
  } catch (err) {
    self.postMessage({
      id: (msg as { id?: string }).id,
      ok: false,
      error: err instanceof Error ? err.message : String(err),
    });
  }
};
