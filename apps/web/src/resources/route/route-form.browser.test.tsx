import { describe, expect, it } from "vitest";
import { render } from "vitest-browser-react";
import { AdminContext } from "@/components/admin";
import { ResourceContextProvider, testDataProvider } from "shadmin-core";
import type { AuthProvider } from "shadmin-core";
import { RouteCreate } from "@/resources/route/route-create";
import { RouteEdit } from "@/resources/route/route-edit";

const MULTI_POINT_RECORD = {
  id: 1,
  name: "Test Route",
  mode: "pokemon",
  geofence_id: 5,
  description: null,
  geometry: { type: "MultiPoint", coordinates: [[1, 2], [3, 4], [5, 6]] },
};

const stubDataProvider = {
  ...testDataProvider({
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    getList: async () => ({ data: [] as any, total: 0 }),
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    getMany: async () => ({ data: [] as any }),
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    getOne: async () => ({ data: MULTI_POINT_RECORD as any }),
  }),
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
    <ResourceContextProvider value="route">{node}</ResourceContextProvider>
  </AdminContext>
);

describe("Route form", () => {
  it("renders name and mode inputs in create view", async () => {
    const screen = render(wrap(<RouteCreate />));
    await expect.element(screen.getByLabelText(/name/i)).toBeVisible();
    await expect.element(screen.getByLabelText(/mode/i)).toBeVisible();
  });

  it("renders a Leaflet map in create view", async () => {
    const screen = render(wrap(<RouteCreate />));
    await expect
      .element(screen.container.querySelector(".leaflet-container"))
      .toBeInTheDocument();
  });

  it("renders the calc workbench (deck map) and keeps the metadata fields in edit view", async () => {
    const screen = render(wrap(<RouteEdit id={1} />));
    await expect.element(screen.getByLabelText(/name/i)).toBeVisible();
    await expect.element(screen.getByLabelText(/mode/i).first()).toBeVisible();
    await expect.element(screen.getByLabelText(/geofence/i)).toBeVisible();
    await expect.element(screen.getByTestId("deck-map")).toBeInTheDocument();
  });
});
