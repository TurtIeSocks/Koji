import { StrictMode, type ReactNode } from "react";
import { createRoot } from "react-dom/client";
import App from "@/App";
import "@/index.css";

function mount(children: ReactNode) {
  createRoot(document.getElementById("root")!).render(
    <StrictMode>{children}</StrictMode>,
  );
}

if (__DEMO__) {
  // Demo boot: dynamically import <DemoBoot> (and everything it pulls in —
  // COI service worker registration, IndexedDB seeding, the wasm calc chain)
  // so none of it ever reaches the live bundle — see demo-boot.tsx's header
  // comment for why plain static imports are safe inside that module once
  // it's reached only through this `if (__DEMO__)` branch. <DemoBoot> mounts
  // immediately and shows its own minimal splash while `ensureSeeded()` runs,
  // rather than blocking the first paint here (Task 8's original approach).
  void (async () => {
    const { DemoBoot } = await import("@/components/demo/demo-boot");
    mount(
      <DemoBoot demo={__DEMO__}>
        <App />
      </DemoBoot>,
    );
  })();
} else {
  mount(<App />);
}
