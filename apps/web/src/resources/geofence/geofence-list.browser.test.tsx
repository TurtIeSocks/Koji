import { describe, expect, it } from "vitest";
import { render } from "vitest-browser-react";
import { AdminContext } from "@/components/admin";
import {
  ResourceContextProvider,
  ListContextProvider,
  testDataProvider,
} from "shadmin-core";
import type { AuthProvider } from "shadmin-core";
import {
  GeofenceList,
  GeofenceBulkToolbar,
} from "@/resources/geofence/geofence-list";

const fakeRows = [
  { id: 1, name: "Alpha", mode: "pokemon", parent: null, geo_type: "Polygon" },
  { id: 2, name: "Beta", mode: "quest", parent: 1, geo_type: "MultiPolygon" },
];

const stubDataProvider = {
  ...testDataProvider({
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    getList: async () => ({ data: fakeRows as any, total: fakeRows.length }),
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    getMany: async () => ({ data: [] as any }),
  }),
  // ListLive → useSubscribe requires subscribe(); no-op for tests
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
  <AdminContext
    dataProvider={stubDataProvider}
    authProvider={stubAuthProvider}
  >
    <ResourceContextProvider value="geofence">{node}</ResourceContextProvider>
  </AdminContext>
);

describe("GeofenceList", () => {
  it("renders geofence rows from the row list", async () => {
    const screen = render(wrap(<GeofenceList />));
    await expect.element(screen.getByText("Alpha")).toBeVisible();
    await expect.element(screen.getByText("Beta")).toBeVisible();
  });

  it("renders the live-search filter input", async () => {
    const screen = render(wrap(<GeofenceList />));
    await expect.element(screen.getByPlaceholder(/search/i)).toBeVisible();
  });

  it("renders an Import action linking to /import", async () => {
    const screen = render(wrap(<GeofenceList />));
    const importLink = screen.getByRole("link", { name: /import/i });
    await expect.element(importLink).toBeVisible();
    // Router-agnostic: hash history renders "#/import", browser history "/import".
    const href = (importLink.element() as HTMLAnchorElement).getAttribute("href");
    expect(href).toMatch(/\/import$/);
  });

  it("renders BulkPublishButton in the bulk toolbar when rows are selected", async () => {
    const screen = render(
      <AdminContext
        dataProvider={stubDataProvider}
        authProvider={stubAuthProvider}
      >
        <ResourceContextProvider value="geofence">
          <ListContextProvider
            value={
              {
                selectedIds: [1],
                onUnselectItems: () => undefined,
              } as any // eslint-disable-line @typescript-eslint/no-explicit-any
            }
          >
            <GeofenceBulkToolbar />
          </ListContextProvider>
        </ResourceContextProvider>
      </AdminContext>,
    );
    await expect
      .element(screen.getByRole("button", { name: /publish/i }))
      .toBeVisible();
  });
});
