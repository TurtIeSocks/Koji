// Shared wasm-module loader for the demo world. `@koji-wasm` (the wasm-pack
// output of crates/koji-wasm) is dynamically imported so it never lands in
// the live bundle, and initialised exactly once — the returned module object
// is cached and every caller (this task's `postConvert`, Task 8's calc trio)
// awaits the same singleton.
//
// The wasm exports a `default` init function (`__wbg_init`), the named
// entry points (`convert_geometry`, `cluster`, `calc`, `s2_cells`, ...), and
// `initThreadPool` for the rayon thread pool. The thread pool only works when
// cross-origin isolation is in effect (SharedArrayBuffer available); we guard
// its init so a non-isolated context still gets a working single-threaded
// module rather than a throw.

type WasmModule = typeof import("@koji-wasm");

let wasmPromise: Promise<WasmModule> | null = null;

async function initWasm(): Promise<WasmModule> {
  const mod = (await import("@koji-wasm")) as WasmModule & {
    default: (input?: unknown) => Promise<unknown>;
  };
  // Instantiate the wasm binary (idempotent per module instance).
  await mod.default();
  // Surface Rust panics as readable console errors when available.
  (mod as { start?: () => void }).start?.();
  // Spin up the rayon thread pool only where SharedArrayBuffer exists — a
  // non-cross-origin-isolated page (e.g. the plain GitHub Pages demo) has none,
  // and calling initThreadPool there would throw. Single-threaded still works.
  if (typeof SharedArrayBuffer !== "undefined") {
    const threads =
      typeof navigator !== "undefined" && navigator.hardwareConcurrency
        ? navigator.hardwareConcurrency
        : 4;
    try {
      await mod.initThreadPool(threads);
    } catch {
      // Thread-pool init is best-effort; fall back to single-threaded.
    }
  }
  return mod;
}

/** The initialised `@koji-wasm` module, loaded and instantiated once. */
export function getWasm(): Promise<WasmModule> {
  if (!wasmPromise) wasmPromise = initWasm();
  return wasmPromise;
}
