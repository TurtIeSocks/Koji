import { act, type ReactNode } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import { AdminContext } from "@/components/admin";
import { ResourceContextProvider, testDataProvider } from "shadmin-core";
import type { AuthProvider } from "shadmin-core";

// `RouteShowMap` mounts the full deck.gl/MapLibre WebGL stack — not viable
// under jsdom. Stub it so this stays a fast unit test (`bun run test
// route-show`), not the browser provider. This is the same path route-show.tsx
// imports directly (not via the `@/components/deck` barrel), so the mock
// target matches exactly.
vi.mock("@/components/deck/route-show-map", () => ({
  RouteShowMap: () => <div data-testid="route-show-map-stub" />,
}));

import { RouteShow } from "@/resources/route/route-show";

(globalThis as unknown as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

// jsdom has no `window.matchMedia` — the admin Show chrome's sidebar uses
// `useIsMobile` (src/hooks/use-mobile.ts), which calls it unconditionally on
// mount. Minimal stub so mounting doesn't crash with an ErrorBoundary.
if (typeof window.matchMedia !== "function") {
  window.matchMedia = ((query: string) => ({
    matches: false,
    media: query,
    onchange: null,
    addListener: () => undefined,
    removeListener: () => undefined,
    addEventListener: () => undefined,
    removeEventListener: () => undefined,
    dispatchEvent: () => false,
  })) as unknown as typeof window.matchMedia;
}

const record = {
  id: 1,
  name: "Rijen_mon",
  mode: "pokemon",
  description: "d",
  geofence_id: 1,
};

const stubDataProvider = {
  ...testDataProvider({
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    getOne: async () => ({ data: record as any }),
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    getList: async () => ({ data: [] as any, total: 0 }),
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    getMany: async () => ({ data: [] as any }),
  }),
  // subscribe() required by ShowLive's realtime context.
  subscribe: () => () => undefined,
};

const stubAuthProvider: AuthProvider = {
  login: async () => undefined,
  logout: async () => undefined,
  checkAuth: async () => undefined,
  checkError: async () => undefined,
  getPermissions: async () => "admin",
  canAccess: async () => true,
};

// This project has no `@testing-library/react` dependency (only
// `@testing-library/jest-dom` for matchers) — hand-roll a minimal render
// backed by `react-dom/client` + React 19's own `act`, mirroring
// deck-geojson-input.test.tsx / use-deck-edit-rhf.test.tsx.
function renderRouteShow() {
  const container = document.createElement("div");
  document.body.appendChild(container);
  let root: Root;
  act(() => {
    root = createRoot(container);
    root.render(
      <AdminContext dataProvider={stubDataProvider} authProvider={stubAuthProvider}>
        <ResourceContextProvider value="route">
          <RouteShow id={1} />
        </ResourceContextProvider>
      </AdminContext> as ReactNode,
    );
  });
  return {
    container,
    // The body wrapper rendered by RouteShow itself (className "p-4"),
    // scoped separately from the surrounding title/breadcrumb chrome that
    // `Show` renders around it — that chrome legitimately repeats the
    // record name, only the body must not.
    body: () => container.querySelector<HTMLDivElement>(".p-4"),
    unmount: () => act(() => root.unmount()),
  };
}

let mounted: ReturnType<typeof renderRouteShow> | null = null;
afterEach(() => {
  mounted?.unmount();
  mounted = null;
});

// The record loads through react-query, whose resolution isn't reliably
// flushed by a fixed number of microtask/macrotask ticks under jsdom — poll
// until the expected content shows up (or time out).
async function waitFor(assertion: () => void) {
  // Wrapped in `act` so the state updates react-query fires as the pending
  // getOne() resolves (outside any event we control) don't log "not wrapped
  // in act" noise.
  await act(async () => {
    await vi.waitFor(assertion, { timeout: 2000, interval: 20 });
  });
}

describe("RouteShow", () => {
  it("does not repeat the record name in the body (breadcrumb/title already show it)", async () => {
    mounted = renderRouteShow();
    // Wait for the record to actually load (proxied by the mode field
    // resolving) before asserting on absence — otherwise "not present yet"
    // and "correctly removed" are indistinguishable.
    await waitFor(() => expect(mounted?.body()?.textContent).toContain("Pokémon"));
    expect(mounted.body()?.textContent).not.toMatch(/Rijen_mon/);
  });

  it("renders mode as the friendly label, not the raw enum", async () => {
    mounted = renderRouteShow();
    await waitFor(() => expect(mounted?.body()?.textContent).toContain("Pokémon"));
    expect(mounted.body()?.textContent).not.toContain("pokemon");
  });

  it("shows a visible 'Mode' label", async () => {
    mounted = renderRouteShow();
    await waitFor(() => expect(mounted?.body()?.textContent).toContain("Mode"));
  });

  it("swaps the geometry field for RouteShowMap", async () => {
    mounted = renderRouteShow();
    await waitFor(() =>
      expect(mounted?.container.querySelector('[data-testid="route-show-map-stub"]')).not.toBeNull(),
    );
  });
});
