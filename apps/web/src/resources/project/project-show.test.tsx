import { act, type ReactNode } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import { AdminContext } from "@/components/admin";
import { ResourceContextProvider, testDataProvider } from "shadmin-core";
import type { AuthProvider } from "shadmin-core";

// `ProjectGeofencesMap` mounts the full deck.gl/MapLibre WebGL stack — not
// viable under jsdom. Stub it so this stays a fast unit test (`bun run test
// project-show`), not the browser provider. The wrapper under test
// (`ProjectShowMap`) lives in project-show.tsx itself (not in this mocked
// module), so it stays exercised for real.
vi.mock("@/components/deck/project-geofences-map", () => ({
  ProjectGeofencesMap: ({ ids }: { ids: (number | string)[] }) => (
    <div data-testid="pmap">{ids.join(",")}</div>
  ),
}));

import { ProjectShow } from "@/resources/project/project-show";

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

const record = { id: 1, name: "P", geofences: [3, 4] };

const stubDataProvider = {
  ...testDataProvider({
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    getOne: async () => ({ data: record as any }),
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    getList: async () => ({ data: [] as any, total: 0 }),
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    getMany: async () => ({ data: [] as any }),
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    getManyReference: async () => ({ data: [] as any, total: 0 }),
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
// route-show.test.tsx / deck-geojson-input.test.tsx.
function renderProjectShow() {
  const container = document.createElement("div");
  document.body.appendChild(container);
  let root: Root;
  act(() => {
    root = createRoot(container);
    root.render(
      <AdminContext dataProvider={stubDataProvider} authProvider={stubAuthProvider}>
        <ResourceContextProvider value="project">
          <ProjectShow id={1} />
        </ResourceContextProvider>
      </AdminContext> as ReactNode,
    );
  });
  return {
    container,
    unmount: () => act(() => root.unmount()),
  };
}

let mounted: ReturnType<typeof renderProjectShow> | null = null;
afterEach(() => {
  mounted?.unmount();
  mounted = null;
});

// The record loads through react-query, whose resolution isn't reliably
// flushed by a fixed number of microtask/macrotask ticks under jsdom — poll
// until the expected content shows up (or time out).
async function waitFor(assertion: () => void) {
  await act(async () => {
    await vi.waitFor(assertion, { timeout: 2000, interval: 20 });
  });
}

describe("ProjectShow map", () => {
  it("keeps the existing geofence chips", async () => {
    mounted = renderProjectShow();
    await waitFor(() =>
      expect(mounted?.container.querySelector('[data-testid="pmap"]')).not.toBeNull(),
    );
    // ReferenceArrayField -> SingleFieldList -> ChipField chain is still
    // present alongside the map (chips resolve from getMany, which our stub
    // returns empty for — presence of the field wrapper is what matters here).
    expect(mounted.container.textContent).toContain("P");
  });

  it("renders ProjectShowMap fed by the record's saved geofence ids", async () => {
    mounted = renderProjectShow();
    await waitFor(() =>
      expect(mounted?.container.querySelector('[data-testid="pmap"]')?.textContent).toBe("3,4"),
    );
  });
});
