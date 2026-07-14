// Tailwind's utility classes only compile into a real stylesheet when this
// global CSS entrypoint is imported — the "browser" vitest project has no
// shared setupFiles.
import "@/index.css";
import { describe, expect, it, vi } from "vitest";
import { render } from "vitest-browser-react";
import { MemoryRouter, Routes, Route } from "react-router";
import { AdminContext } from "@/components/admin";
import { ResourceContextProvider, testDataProvider } from "shadmin-core";
import type { AuthProvider } from "shadmin-core";
import { ProjectCreate } from "@/resources/project/project-create";
import { ProjectEdit } from "@/resources/project/project-edit";

// Stub the member-geofences map entirely — this suite is about TabbedForm
// wiring (tab triggers, which panel is visible), not the map itself.
// Mounting the real ProjectGeofencesMap would pull in deck.gl/maplibre for
// no test value here.
// Content (not an empty div) matters: an empty div collapses to a 0x0 box,
// which reads as "not visible" to the visibility matcher below even once its
// tabpanel's `display:none` is lifted.
vi.mock("@/components/deck/project-geofences-map", () => ({
  ProjectGeofencesMap: () => <div data-testid="project-map-stub">map</div>,
}));

const EDIT_RECORD = {
  id: 1,
  name: "Test Project",
  description: null,
  geofences: [5],
};

const stubDataProvider = {
  ...testDataProvider({
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    getList: async () => ({ data: [] as any, total: 0 }),
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    getMany: async () => ({ data: [{ id: 5, name: "Fence" }] as any }),
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    getOne: async () => ({ data: EDIT_RECORD as any }),
  }),
  // subscribe() required by realtime context providers (ProjectEdit uses EditLive)
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

// TabbedForm syncs the active tab to the URL via useNavigate/useLocation.
// AdminContext falls back to a real HashRouter bound to the browser's actual
// address bar when it isn't already inside a router — that would leak the
// active tab's URL across tests in this file (a real browser, not jsdom, so
// the hash genuinely persists between renders). Wrap each render in its own
// MemoryRouter so every test starts from a clean, isolated location.
//
// A bare <MemoryRouter> isn't enough, though: TabbedForm derives its tab
// paths from useParams()['*'], which only resolves to "" (vs. undefined)
// when rendered under a matched splat Route — exactly how react-admin's real
// resource routing mounts `create/*` and `:id/*` pages. Without that Route,
// splatPathBase computes with a leading "/" instead of "", so the Map tab's
// path becomes "//1" instead of "/1" — and react-router's matchPath doesn't
// match that pattern against itself. Wrapping in <Routes><Route path="/*">
// reproduces the real app's routing shape and avoids that double-slash
// mismatch.
const wrap = (node: React.ReactNode) => (
  <MemoryRouter>
    <Routes>
      <Route
        path="/*"
        element={
          <AdminContext dataProvider={stubDataProvider} authProvider={stubAuthProvider}>
            <ResourceContextProvider value="project">{node}</ResourceContextProvider>
          </AdminContext>
        }
      />
    </Routes>
  </MemoryRouter>
);

describe("ProjectCreate — TabbedForm", () => {
  it("renders Details and Map tab triggers", async () => {
    const screen = render(wrap(<ProjectCreate />));
    await expect
      .element(screen.getByRole("tab", { name: "Details" }))
      .toBeVisible();
    await expect
      .element(screen.getByRole("tab", { name: "Map" }))
      .toBeVisible();
  });

  it("shows the Details tab (name + geofences input) active by default, Map tab panel hidden", async () => {
    const screen = render(wrap(<ProjectCreate />));
    await expect.element(screen.getByLabelText(/name/i)).toBeVisible();
    // shadmin's AutocompleteArrayInput renders a "Geofences" <label> but
    // without the for/id association getByLabelText needs, so match the
    // label text directly (mirrors the geofence form's "Projects" field).
    await expect.element(screen.getByText("Geofences")).toBeVisible();
    // The map stub is mounted (all tabs render, per TabbedForm's design) but its
    // panel is the inactive one — getByRole excludes aria-hidden content by
    // default, so it must NOT be found via the accessibility tree yet.
    await expect
      .element(screen.getByRole("tabpanel", { name: "Map" }))
      .not.toBeInTheDocument();
    // It's still present in the DOM, just hidden.
    expect(screen.container.querySelector('[data-testid="project-map-stub"]')).not.toBeNull();
  });

  it("reveals the project-map-stub on the Map tab's panel after clicking it", async () => {
    const screen = render(wrap(<ProjectCreate />));
    await expect
      .element(screen.getByRole("tab", { name: "Map" }))
      .toBeVisible();
    await screen.getByRole("tab", { name: "Map" }).click();

    await expect
      .element(screen.getByRole("tabpanel", { name: "Map" }))
      .toBeVisible();
    await expect
      .element(screen.getByTestId("project-map-stub"))
      .toBeVisible();
  });
});

describe("ProjectEdit — TabbedForm", () => {
  it("keeps the geofences input inside the Details tab, visible without switching tabs", async () => {
    const screen = render(wrap(<ProjectEdit id={1} />));
    await expect.element(screen.getByLabelText(/name/i)).toBeVisible();
    await expect.element(screen.getByText("Geofences")).toBeVisible();
  });
});
