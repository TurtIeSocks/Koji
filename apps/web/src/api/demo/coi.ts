// Cross-Origin-Isolation bootstrap for the demo. The wasm rayon thread pool
// needs SharedArrayBuffer, which the browser only exposes when the page is
// `crossOriginIsolated` (COOP: same-origin + COEP: credentialless/require-corp).
//
// The dev server (`vite --mode demo`) sets those headers directly, so isolation
// is already in effect there and this is a no-op. On a static host (GitHub
// Pages) headers can't be set, so we register the vendored `coi-serviceworker.js`
// (public/, root scope): it injects the headers into every response and reloads
// the page once so the reload lands under isolation. Registration is gated on
// `__DEMO__` — the live app must NOT install this SW.

/** Register the COI service worker when running the demo without cross-origin
 *  isolation. Safe to call unconditionally from the demo boot: it early-returns
 *  in the live build, in non-browser/insecure contexts, and when the page is
 *  already isolated. The vendored SW self-reloads the page once on first visit
 *  to gain isolation. */
export function maybeRegisterCoi(): void {
  if (!__DEMO__) return;
  if (typeof window === "undefined") return;
  // Already isolated (dev headers, or a prior SW-driven reload) → nothing to do.
  if (window.crossOriginIsolated) return;
  // A service worker needs a secure context (https or localhost).
  if (!window.isSecureContext) return;
  const sw = navigator.serviceWorker;
  if (!sw) return;

  // Tell an already-controlling SW to use credentialless COEP (keeps
  // cross-origin basemap tiles loadable). On the very first visit there's no
  // controller yet — the vendored SW defaults to credentialless anyway.
  sw.controller?.postMessage({ type: "coepCredentialless", value: true });

  sw.register(`${import.meta.env.BASE_URL}coi-serviceworker.js`).then(
    (registration) => {
      registration.addEventListener("updatefound", () => {
        window.sessionStorage.setItem("coiReloadedBySelf", "updatefound");
        window.location.reload();
      });
      // Active SW that isn't yet controlling this page → reload so the next load
      // is served through it (and thus cross-origin isolated).
      if (registration.active && !sw.controller) {
        window.sessionStorage.setItem("coiReloadedBySelf", "notcontrolling");
        window.location.reload();
      }
    },
    (err) => console.error("COOP/COEP Service Worker failed to register:", err),
  );
}

/** Heuristic for Task 9's "isolation unavailable" banner: the COI service worker
 *  is registered/controlling yet the page is still not cross-origin isolated
 *  (e.g. a browser that ignores the injected headers). When true, the wasm
 *  thread pool is off and calcs run single-threaded — correct, just slower. */
export function isolationFailed(): boolean {
  if (typeof window === "undefined") return false;
  if (window.crossOriginIsolated) return false;
  return Boolean(navigator.serviceWorker?.controller);
}
