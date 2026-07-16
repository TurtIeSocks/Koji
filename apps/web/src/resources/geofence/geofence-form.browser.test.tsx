import { describe, expect, it } from "vitest";
import { render } from "vitest-browser-react";
import { AdminContext } from "@/components/admin";
import { ResourceContextProvider, testDataProvider } from "shadmin-core";
import type { AuthProvider } from "shadmin-core";
import { GeofenceCreate } from "@/resources/geofence/geofence-create";
import { GeofenceEdit } from "@/resources/geofence/geofence-edit";

const MULTI_POLYGON_RECORD = {
  id: 1,
  name: "Test MultiPolygon",
  mode: "unset",
  parent: null,
  projects: [10],
  geometry: {
    type: "MultiPolygon" as const,
    coordinates: [
      [[[0, 0], [1, 0], [1, 1], [0, 1], [0, 0]]],
      [[[2, 2], [3, 2], [3, 3], [2, 3], [2, 2]]],
    ],
  },
};

const stubDataProvider = {
  ...testDataProvider({
    getList: async (resource: string) =>
      resource === "property"
        ? // eslint-disable-next-line @typescript-eslint/no-explicit-any
          { data: [{ id: 10, name: "is_event", category: "boolean" }] as any, total: 1 }
        : // eslint-disable-next-line @typescript-eslint/no-explicit-any
          { data: [] as any, total: 0 },
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    getMany: async () => ({ data: [{ id: 10, name: "ProjectAlpha" }] as any }),
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    getOne: async () => ({ data: MULTI_POLYGON_RECORD as any }),
  }),
  // subscribe() required by realtime context providers
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

const wrap = (node: React.ReactNode) => (
  <AdminContext dataProvider={stubDataProvider} authProvider={stubAuthProvider}>
    <ResourceContextProvider value="geofence">{node}</ResourceContextProvider>
  </AdminContext>
);

describe("Geofence form", () => {
  it("renders name and mode inputs", async () => {
    const screen = render(wrap(<GeofenceCreate />));
    await expect.element(screen.getByLabelText(/name/i)).toBeVisible();
    await expect.element(screen.getByLabelText(/mode/i)).toBeVisible();
  });

  it("renders the deck geometry map (NOT Leaflet) in the create form", async () => {
    const screen = render(wrap(<GeofenceCreate />));
    await expect.element(screen.getByTestId("deck-map")).toBeInTheDocument();
    expect(screen.container.querySelector(".leaflet-container")).toBeNull();
  });

  it("renders the deck geometry map (NOT Leaflet) in the edit form for a MultiPolygon record", async () => {
    // Edit uses the deck <GeofenceMap>, not Leaflet. Pass id={1} so
    // useEditController resolves MULTI_POLYGON_RECORD (no route params in test);
    // the name input appearing proves the async getOne resolved and the form mounted.
    const screen = render(wrap(<GeofenceEdit id={1} />));
    await expect.element(screen.getByLabelText(/name/i)).toBeVisible();
    // The deck map must mount for the MultiPolygon record...
    await expect.element(screen.getByTestId("deck-map")).toBeInTheDocument();
    // ...and there must be NO Leaflet map on the edit page (single-map requirement).
    expect(screen.container.querySelector(".leaflet-container")).toBeNull();
  });

  // shadmin's AutocompleteArrayInput renders a "Projects" <label> but without the
  // for/id association getByLabelText needs, so match the label text directly.
  // Split into two single-render tests — rendering two <AdminContext> + leaflet
  // forms in one test double-mounts the query client and the first never settles.
  it("renders the projects autocomplete in Edit", async () => {
    const screen = render(wrap(<GeofenceEdit id={1} />));
    await expect.element(screen.getByText("Projects")).toBeVisible();
  });

  it("renders the projects autocomplete in Create", async () => {
    const screen = render(wrap(<GeofenceCreate />));
    await expect.element(screen.getByLabelText(/name/i)).toBeVisible();
    await expect.element(screen.getByText("Projects")).toBeVisible();
  });

  it("renders the properties array input in the geofence form", async () => {
    const screen = render(wrap(<GeofenceCreate />));
    await expect.element(screen.getByLabelText(/name/i)).toBeVisible();
    // ArrayInput for properties renders the add control.
    await expect
      .element(screen.container.querySelector(".button-add-properties"))
      .toBeInTheDocument();
  });
});
