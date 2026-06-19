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
import { AssignProjectsBulkButton } from "./assign-projects-bulk";

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
          selectedIds: [5, 6],
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

describe("AssignProjectsBulkButton", () => {
  it("renders the trigger button", async () => {
    const screen = render(wrap(<AssignProjectsBulkButton />));
    await expect
      .element(screen.getByRole("button", { name: /assign projects/i }))
      .toBeVisible();
  });

  it("opens the dialog on click", async () => {
    const screen = render(wrap(<AssignProjectsBulkButton />));
    const trigger = screen.getByRole("button", { name: /assign projects/i });
    await userEvent.click(trigger);
    await expect
      .element(screen.getByRole("dialog", { name: /assign projects/i }))
      .toBeVisible();
  });
});
