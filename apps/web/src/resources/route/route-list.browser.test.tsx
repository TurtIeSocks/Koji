import { describe, expect, it } from "vitest";
import { render } from "vitest-browser-react";
import { AdminContext } from "@/components/admin";
import {
  ResourceContextProvider,
  ListContextProvider,
  testDataProvider,
} from "shadmin-core";
import type { AuthProvider } from "shadmin-core";
import { RouteList, RouteBulkToolbar } from "@/resources/route/route-list";

const fakeRows = [
  { id: 1, name: "North Loop", mode: "pokemon", geofence_id: 10, points: 42, description: null },
  { id: 2, name: "South Run", mode: "fort", geofence_id: 11, points: 7, description: "daily" },
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

const wrap = (node: React.ReactNode) => (
  <AdminContext dataProvider={stubDataProvider} authProvider={stubAuthProvider}>
    <ResourceContextProvider value="route">{node}</ResourceContextProvider>
  </AdminContext>
);

describe("RouteList", () => {
  it("renders route rows from the row list", async () => {
    const screen = render(wrap(<RouteList />));
    await expect.element(screen.getByText("North Loop")).toBeVisible();
    await expect.element(screen.getByText("South Run")).toBeVisible();
  });

  it("renders the live-search filter input", async () => {
    const screen = render(wrap(<RouteList />));
    await expect.element(screen.getByPlaceholder(/search/i)).toBeVisible();
  });

  it("renders BulkPublishButton in the bulk toolbar when rows are selected", async () => {
    const screen = render(
      <AdminContext dataProvider={stubDataProvider} authProvider={stubAuthProvider}>
        <ResourceContextProvider value="route">
          <ListContextProvider
            value={
              {
                selectedIds: [1],
                onUnselectItems: () => undefined,
              } as any // eslint-disable-line @typescript-eslint/no-explicit-any
            }
          >
            <RouteBulkToolbar />
          </ListContextProvider>
        </ResourceContextProvider>
      </AdminContext>,
    );
    await expect
      .element(screen.getByRole("button", { name: /publish/i }))
      .toBeVisible();
  });
});
