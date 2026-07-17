import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import App from "@/App";
import "@/index.css";

function mount() {
  createRoot(document.getElementById("root")!).render(
    <StrictMode>
      <App />
    </StrictMode>,
  );
}

if (__DEMO__) {
  // Demo boot: (1) register the COI service worker so a static host gains the
  // cross-origin isolation the wasm thread pool needs (no-op under the dev
  // server, which already sets the headers — and it may reload the page once on
  // first visit); (2) seed the synthetic world before first paint so the admin
  // lists/map aren't briefly empty; (3) mount; (4) seed demo routes in the
  // background (needs the wasm worker, so never block paint on it). The demo-only
  // modules are dynamically imported behind `__DEMO__` so the live bundle never
  // pulls them in. Task 9 replaces this minimal gate with a splash screen.
  void (async () => {
    const { maybeRegisterCoi } = await import("@/api/demo/coi");
    maybeRegisterCoi();
    const { ensureSeeded, seedRoutes } = await import("@/api/demo/seeds/seed");
    const { submitCalc, getJob } = await import("@/api/demo/calc/facade");
    await ensureSeeded();
    mount();
    void seedRoutes(submitCalc, getJob);
  })();
} else {
  mount();
}
