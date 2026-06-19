import { describe, expect, it } from "vitest";
import { render } from "vitest-browser-react";
import { userEvent } from "@vitest/browser/context";
import { AdminContext } from "@/components/admin";
import {
  ResourceContextProvider,
  testDataProvider,
  ListContextProvider,
} from "shadmin-core";
import type { AuthProvider } from "shadmin-core";
import { AssignParentBulkButton } from "./assign-parent-bulk";

const stubDataProvider = {
  ...testDataProvider({
    getList: async () => ({ data: [] as any, total: 0 }),
    getMany: async () => ({ data: [] as any }),
    updateMany: async () => ({ data: [] as any }),
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
    <ResourceContextProvider value="geofence">
      <ListContextProvider
        value={{
          selectedIds: [3, 4],
          data: [],
          total: 0,
          page: 1,
          perPage: 10,
          sort: { field: "id", order: "ASC" as const },
          filter: {},
          filterValues: {},
          displayedFilters: {},
          showFilter: () => undefined,
          hideFilter: () => undefined,
          setFilters: () => undefined,
          setPage: () => undefined,
          setPerPage: () => undefined,
          setSort: () => undefined,
          onSelect: () => undefined,
          onToggleItem: () => undefined,
          onUnselectItems: () => undefined,
          isPending: false,
          isFetching: false,
          isLoading: false,
          resource: "geofence",
          refetch: () => undefined as any,
        }}
      >
        {node}
      </ListContextProvider>
    </ResourceContextProvider>
  </AdminContext>
);

describe("AssignParentBulkButton", () => {
  it("renders the trigger button", async () => {
    const screen = render(wrap(<AssignParentBulkButton />));
    await expect
      .element(screen.getByRole("button", { name: /assign parent/i }))
      .toBeVisible();
  });

  it("opens the dialog on click", async () => {
    const screen = render(wrap(<AssignParentBulkButton />));
    const trigger = screen.getByRole("button", { name: /assign parent/i });
    await userEvent.click(trigger);
    await expect.element(screen.getByRole("dialog")).toBeVisible();
  });
});
