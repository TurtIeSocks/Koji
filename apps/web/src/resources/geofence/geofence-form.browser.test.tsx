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

  it("renders an interactive Leaflet map with geoman controls", async () => {
    const screen = render(wrap(<GeofenceCreate />));
    // leaflet-container is the standard Leaflet root div
    await expect
      .element(screen.container.querySelector(".leaflet-container"))
      .toBeInTheDocument();
    // GeomanControls renders the geoman toolbar (react-leaflet-geoman-v2)
    await expect
      .element(screen.container.querySelector(".leaflet-pm-toolbar"))
      .toBeInTheDocument();
  });

  it("renders the geometry map without error when a MultiPolygon record is loaded", async () => {
    // GeofenceEdit hydrates the map from the record's geometry. If PolygonInput
    // were still in place it would coerce MultiPolygon to Polygon on save,
    // silently dropping sub-polygons. MultiPolygonInput must hydrate both rings.
    // Pass id={1} so useEditController resolves the record (no route params in test).
    const screen = render(wrap(<GeofenceEdit id={1} />));
    // Wait for the record to load — EditView gates children on context.record,
    // so the name input appearing proves the async getOne resolved and the form
    // (including the Leaflet map) has mounted.
    await expect.element(screen.getByLabelText(/name/i)).toBeVisible();
    // Map must mount — no thrown error or missing container.
    await expect
      .element(screen.container.querySelector(".leaflet-container"))
      .toBeInTheDocument();
    // Geoman toolbar must also be present.
    await expect
      .element(screen.container.querySelector(".leaflet-pm-toolbar"))
      .toBeInTheDocument();
  });

  it("renders the deck geometry input in the edit form", async () => {
    const screen = render(wrap(<GeofenceEdit id={1} />));
    await expect.element(screen.getByLabelText(/name/i)).toBeVisible();
    await expect.element(screen.getByTestId("deck-map")).toBeInTheDocument();
  });

  // shadmin's AutocompleteArrayInput renders a "Projects" <label> but without the
  // for/id association getByLabelText needs, so match the label text directly.
  // Split into two single-render tests — rendering two <AdminContext> + leaflet
  // forms in one test double-mounts the query client and the first never settles.
  it("renders the projects autocomplete in Edit", async () => {
    const screen = render(wrap(<GeofenceEdit id={1} />));
    await expect.element(screen.getByText("Projects")).toBeVisible();
  });

  it("does NOT render projects in Create", async () => {
    const screen = render(wrap(<GeofenceCreate />));
    await expect.element(screen.getByLabelText(/name/i)).toBeVisible();
    expect(
      [...screen.container.querySelectorAll("label")].some((l) =>
        /projects/i.test(l.textContent ?? ""),
      ),
    ).toBe(false);
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
