import { describe, expect, it } from "vitest";
import { render } from "vitest-browser-react";
import { AdminContext } from "@/components/admin";
import { ResourceContextProvider, testDataProvider } from "shadmin-core";
import type { AuthProvider } from "shadmin-core";
import { PropertyList } from "@/resources/property/property-list";

const fakeRows = [
  { id: 1, name: "spawn_color", category: "color", default_value: "#ff0000", geofences: [1] },
  { id: 2, name: "iv_min", category: "number", default_value: 80, geofences: [] },
];

const stubDataProvider = {
  ...testDataProvider({
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    getList: async () => ({ data: fakeRows as any, total: fakeRows.length }),
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    getMany: async () => ({ data: [] as any }),
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

describe("PropertyList", () => {
  it("renders property rows with name and category", async () => {
    const screen = render(
      <AdminContext dataProvider={stubDataProvider} authProvider={stubAuthProvider}>
        <ResourceContextProvider value="property">
          <PropertyList />
        </ResourceContextProvider>
      </AdminContext>,
    );
    await expect.element(screen.getByText("spawn_color")).toBeVisible();
    await expect.element(screen.getByText("iv_min")).toBeVisible();
  });
});
