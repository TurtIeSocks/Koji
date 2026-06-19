import { describe, expect, it } from "vitest";
import { render } from "vitest-browser-react";
import { AdminContext } from "@/components/admin";
import { ResourceContextProvider, testDataProvider } from "shadmin-core";
import type { AuthProvider } from "shadmin-core";
import { GeofenceShow } from "@/resources/geofence/geofence-show";

const record = {
  id: 1,
  name: "Alpha",
  mode: "pokemon",
  geo_type: "Polygon",
  parent: null,
  projects: [20],
  geometry: { type: "Polygon", coordinates: [[[0, 0], [0, 1], [1, 1], [0, 0]]] },
};

const stubDataProvider = {
  ...testDataProvider({
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    getOne: async () => ({ data: record as any }),
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    getList: async () => ({ data: [] as any, total: 0 }),
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    getMany: async () => ({ data: [{ id: 20, name: "ProjectBeta" }] as any }),
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

describe("GeofenceShow", () => {
  it("renders the read-only geometry map and field values", async () => {
    const screen = render(
      <AdminContext dataProvider={stubDataProvider} authProvider={stubAuthProvider}>
        <ResourceContextProvider value="geofence">
          <GeofenceShow id={1} />
        </ResourceContextProvider>
      </AdminContext>,
    );
    await expect.element(screen.getByText("Alpha", { exact: true }).first()).toBeVisible();
    await expect
      .element(screen.container.querySelector(".leaflet-container"))
      .toBeInTheDocument();
  });

  it("renders project chips in the projects field", async () => {
    const screen = render(
      <AdminContext dataProvider={stubDataProvider} authProvider={stubAuthProvider}>
        <ResourceContextProvider value="geofence">
          <GeofenceShow id={1} />
        </ResourceContextProvider>
      </AdminContext>,
    );
    await expect.element(screen.getByText("ProjectBeta")).toBeVisible();
  });
});
