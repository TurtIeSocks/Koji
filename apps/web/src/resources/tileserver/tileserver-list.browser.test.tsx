import { describe, expect, it } from "vitest";
import { render } from "vitest-browser-react";
import { AdminContext } from "@/components/admin";
import { ResourceContextProvider, testDataProvider } from "shadmin-core";
import type { AuthProvider } from "shadmin-core";
import { TileserverList } from "@/resources/tileserver/tileserver-list";

const fakeRows = [
  { id: 1, name: "CartoDB Voyager", url: "https://{s}.basemaps.cartocdn.com/..." },
  { id: 2, name: "OSM Standard", url: "https://{s}.tile.openstreetmap.org/..." },
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

describe("TileserverList", () => {
  it("renders tileserver rows with name and url", async () => {
    const screen = render(
      <AdminContext dataProvider={stubDataProvider} authProvider={stubAuthProvider}>
        <ResourceContextProvider value="tileserver">
          <TileserverList />
        </ResourceContextProvider>
      </AdminContext>,
    );
    await expect.element(screen.getByText("CartoDB Voyager")).toBeVisible();
    await expect.element(screen.getByText("OSM Standard")).toBeVisible();
  });
});
