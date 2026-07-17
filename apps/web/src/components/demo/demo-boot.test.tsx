import { act, type ReactNode } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";

// `DemoBoot` statically imports the demo api surface (coi/seed/calc/db) —
// fine in the real app (this module is only ever reached through main.tsx's
// `if (__DEMO__)` dynamic import, see demo-boot.tsx's header comment), but a
// unit test wants fakes, not the real IndexedDB/wasm chain. Mock each module
// DemoBoot imports directly.
const maybeRegisterCoi = vi.fn();
const isolationFailedMock = vi.fn(() => false);
vi.mock("@/api/demo/coi", () => ({
  maybeRegisterCoi: () => maybeRegisterCoi(),
  isolationFailed: () => isolationFailedMock(),
}));

const ensureSeeded = vi.fn(() => Promise.resolve());
const seedRoutes = vi.fn((_submitCalc: unknown, _getJob: unknown) => Promise.resolve());
vi.mock("@/api/demo/seeds/seed", () => ({
  ensureSeeded: () => ensureSeeded(),
  seedRoutes: (submitCalc: unknown, getJob: unknown) => seedRoutes(submitCalc, getJob),
}));

vi.mock("@/api/demo/calc/facade", () => ({
  submitCalc: vi.fn(),
  getJob: vi.fn(),
}));

// A getter (not a plain value) so tests can flip it between cases — vitest's
// mock namespace re-reads it on every `import { persistent } from "..."`
// access, mirroring the real module's mutable `export let persistent`.
let mockPersistent = true;
vi.mock("@/api/demo/db", () => ({
  get persistent() {
    return mockPersistent;
  },
}));

const toastWarning = vi.fn();
vi.mock("sonner", () => ({
  toast: { warning: (message: unknown) => toastWarning(message) },
}));

import { DemoBoot } from "@/components/demo/demo-boot";

(globalThis as unknown as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

function renderDemoBoot(demo: boolean, children: ReactNode) {
  const container = document.createElement("div");
  document.body.appendChild(container);
  let root: Root;
  act(() => {
    root = createRoot(container);
    root.render(<DemoBoot demo={demo}>{children}</DemoBoot>);
  });
  return {
    container,
    unmount: () => act(() => root.unmount()),
  };
}

async function flush() {
  await act(async () => {
    await Promise.resolve();
    await Promise.resolve();
  });
}

let mounted: ReturnType<typeof renderDemoBoot> | null = null;
afterEach(() => {
  mounted?.unmount();
  mounted = null;
  mockPersistent = true;
  vi.clearAllMocks();
});

describe("DemoBoot", () => {
  it("renders children immediately when demo is false", () => {
    mounted = renderDemoBoot(false, <div data-testid="child">app</div>);
    expect(mounted.container.querySelector('[data-testid="child"]')).not.toBeNull();
    expect(mounted.container.textContent).not.toContain("Seeding demo world");
    expect(ensureSeeded).not.toHaveBeenCalled();
    expect(maybeRegisterCoi).not.toHaveBeenCalled();
  });

  it("shows a seeding splash while ensureSeeded is pending, then renders children", async () => {
    let resolveSeed: () => void = () => undefined;
    ensureSeeded.mockImplementationOnce(
      () =>
        new Promise<void>((resolve) => {
          resolveSeed = resolve;
        }),
    );

    mounted = renderDemoBoot(true, <div data-testid="child">app</div>);

    // Splash first — ensureSeeded hasn't resolved yet.
    expect(mounted.container.textContent).toContain("Seeding demo world");
    expect(mounted.container.querySelector('[data-testid="child"]')).toBeNull();
    expect(maybeRegisterCoi).toHaveBeenCalledTimes(1);

    await act(async () => {
      resolveSeed();
      await Promise.resolve();
    });

    // Then children, once ensureSeeded resolves.
    expect(mounted.container.querySelector('[data-testid="child"]')).not.toBeNull();
    expect(mounted.container.textContent).not.toContain("Seeding demo world");
    expect(seedRoutes).toHaveBeenCalledTimes(1);
  });

  it("surfaces a dismissible banner when isolation failed", async () => {
    isolationFailedMock.mockReturnValueOnce(true);
    mounted = renderDemoBoot(true, <div data-testid="child">app</div>);
    await flush();

    expect(mounted.container.textContent).toContain("Cross-origin isolation unavailable");
    const dismiss = mounted.container.querySelector('[aria-label="Dismiss"]') as HTMLButtonElement | null;
    expect(dismiss).not.toBeNull();

    act(() => dismiss!.click());
    expect(mounted.container.textContent).not.toContain("Cross-origin isolation unavailable");
  });

  it("toasts once when the demo world isn't backed by persistent storage", async () => {
    mockPersistent = false;
    mounted = renderDemoBoot(true, <div data-testid="child">app</div>);
    await flush();

    expect(toastWarning).toHaveBeenCalledTimes(1);
  });

  it("does not toast when storage is persistent", async () => {
    mockPersistent = true;
    mounted = renderDemoBoot(true, <div data-testid="child">app</div>);
    await flush();

    expect(toastWarning).not.toHaveBeenCalled();
  });

  it("shows an error screen (not an endless splash) when seeding rejects", async () => {
    ensureSeeded.mockRejectedValueOnce(new Error("idb blocked"));
    mounted = renderDemoBoot(true, <div data-testid="child">app</div>);
    await flush();

    // No infinite spinner, no children — a readable error instead.
    expect(mounted.container.textContent).not.toContain("Seeding demo world");
    expect(mounted.container.querySelector('[data-testid="child"]')).toBeNull();
    expect(mounted.container.textContent).toContain("Couldn't start the demo");
    expect(mounted.container.textContent).toContain("idb blocked");
    expect(seedRoutes).not.toHaveBeenCalled();
  });
});
