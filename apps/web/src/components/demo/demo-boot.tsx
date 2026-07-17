import { useEffect, useRef, useState, type ReactNode } from "react";
import { XIcon } from "lucide-react";
import { toast } from "sonner";
import { isolationFailed, maybeRegisterCoi } from "@/api/demo/coi";
import { getJob, submitCalc } from "@/api/demo/calc/facade";
import { persistent } from "@/api/demo/db";
import { ensureSeeded, seedRoutes } from "@/api/demo/seeds/seed";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { Spinner } from "@/components/ui/spinner";

// This module is only ever reached via main.tsx's `if (__DEMO__) { await
// import("@/components/demo/demo-boot") }` — a live build never bundles it (or
// its imports, incl. the wasm-backed seed/calc chain), so plain static
// imports are fine here, same as seed.ts -> geometry.ts or worker.ts ->
// @koji-wasm. The `demo` prop below exists purely so THIS component's own
// gate logic (splash -> children, banner, toast) is unit-testable in
// isolation (see demo-boot.test.tsx) — it does not control tree-shaking.

interface DemoBootProps {
  /** Whether to run the demo boot sequence at all. Always `true` in the real
   *  app (main.tsx only mounts this component when `__DEMO__`); a unit test
   *  passes `false` directly to exercise the pass-through branch without
   *  needing a `__DEMO__` rebuild. */
  demo: boolean;
  children: ReactNode;
}

type Phase = "seeding" | "ready" | "error";

/** Wraps `<App/>` in the demo build: registers the COI service worker, awaits
 *  `ensureSeeded()` behind a minimal centered splash so the admin lists/map
 *  aren't briefly empty, then renders `children`. Once ready, seeds demo
 *  routes in the background (needs the wasm worker, so it never blocks
 *  paint), surfaces a dismissible banner if cross-origin isolation failed
 *  (calc still works, just single-threaded/slower), and fires a one-time
 *  toast if the IndexedDB-backed world fell back to an in-memory store
 *  (edits won't survive a reload). When `demo` is false, renders `children`
 *  immediately with none of the above. */
function DemoBoot({ demo, children }: DemoBootProps) {
  const [phase, setPhase] = useState<Phase>(demo ? "seeding" : "ready");
  const [seedError, setSeedError] = useState<string | null>(null);
  const [isolationWarning, setIsolationWarning] = useState(false);
  const [bannerDismissed, setBannerDismissed] = useState(false);
  const toastShown = useRef(false);

  useEffect(() => {
    if (!demo) return;
    let cancelled = false;
    void (async () => {
      maybeRegisterCoi();
      try {
        await ensureSeeded();
      } catch (e) {
        // A rejecting IndexedDB (e.g. Safari private mode, where `indexedDB`
        // exists but every transaction throws) would otherwise leave the splash
        // spinning forever. Surface it instead of hanging.
        if (cancelled) return;
        setSeedError(e instanceof Error ? e.message : String(e));
        setPhase("error");
        return;
      }
      if (cancelled) return;
      setIsolationWarning(isolationFailed());
      setPhase("ready");
      void seedRoutes(submitCalc, getJob);
    })();
    return () => {
      cancelled = true;
    };
  }, [demo]);

  useEffect(() => {
    if (!demo || phase !== "ready" || toastShown.current) return;
    if (!persistent) {
      toastShown.current = true;
      toast.warning("This browser has no IndexedDB — edits won't persist across a reload.");
    }
  }, [demo, phase]);

  if (phase === "seeding") {
    return (
      <div className="flex h-svh w-full flex-col items-center justify-center gap-3">
        <Spinner className="size-6" />
        <p className="text-sm text-muted-foreground">Seeding demo world…</p>
      </div>
    );
  }

  if (phase === "error") {
    return (
      <div className="flex h-svh w-full flex-col items-center justify-center gap-3 p-6">
        <Alert variant="destructive" className="max-w-md">
          <AlertTitle>Couldn't start the demo</AlertTitle>
          <AlertDescription>
            <span>
              The demo needs browser storage (IndexedDB), which this browser blocked —
              private-browsing windows in some browsers disable it. Try a normal window.
              {seedError ? ` (${seedError})` : ""}
            </span>
          </AlertDescription>
        </Alert>
      </div>
    );
  }

  return (
    <>
      {isolationWarning && !bannerDismissed && (
        // `fixed` + `z-50` (same overlay layer as dialog.tsx/sheet.tsx) so this
        // floats above the page instead of sitting in normal flow — Layout's
        // <main> is `h-svh` (exactly one viewport tall), so a banner pushing
        // content down here would shove its bottom off-screen.
        <Alert
          variant="destructive"
          className="fixed inset-x-0 top-0 z-50 rounded-none border-x-0 border-t-0 pr-10 shadow-md"
        >
          <AlertTitle>Cross-origin isolation unavailable</AlertTitle>
          <AlertDescription>
            <span>
              This browser didn't pick up the isolation headers the demo needs for
              multi-threaded calculations — calc jobs still run, just single-threaded
              (slower). Everything else works normally.
            </span>
          </AlertDescription>
          <Button
            aria-label="Dismiss"
            className="absolute top-3 right-3"
            onClick={() => setBannerDismissed(true)}
            size="icon-sm"
            variant="ghost"
          >
            <XIcon />
          </Button>
        </Alert>
      )}
      {children}
    </>
  );
}

export { DemoBoot };
