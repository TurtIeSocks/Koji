import { describe, expect, it } from "vitest";
import { render } from "vitest-browser-react";
import { AdminContext } from "@/components/admin";
import { ResourceContextProvider, testDataProvider } from "shadmin-core";
import type { AuthProvider } from "shadmin-core";
import { PluginsList } from "@/resources/plugins/plugins-list";

const fakeRows = [
  { id: "scanner:rdm-bridge", name: "rdm-bridge", kind: "scanner", enabled: true, version: "1.0.0" },
  { id: "export:geojson", name: "geojson", kind: "export", enabled: false, version: "0.9.0" },
];

const stubDataProvider = {
  ...testDataProvider({
    getList: async () => ({ data: fakeRows as any, total: fakeRows.length }),
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

describe("PluginsList", () => {
  it("renders plugin rows with name, kind, and enabled status", async () => {
    const screen = render(
      <AdminContext dataProvider={stubDataProvider} authProvider={stubAuthProvider}>
        <ResourceContextProvider value="plugins">
          <PluginsList />
        </ResourceContextProvider>
      </AdminContext>,
    );
    await expect.element(screen.getByText("rdm-bridge")).toBeVisible();
    await expect.element(screen.getByText("geojson")).toBeVisible();
  });
});
