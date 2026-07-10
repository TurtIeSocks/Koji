import { describe, expect, it } from "vitest";
import { render } from "vitest-browser-react";
import { Routes, Route, useLocation } from "react-router";
import { AdminContext } from "@/components/admin";
import { ResourceContextProvider, testDataProvider } from "shadmin-core";
import type { AuthProvider } from "shadmin-core";
import { CreateButton } from "./create-button";

const authProvider: AuthProvider = {
  login: async () => undefined,
  logout: async () => undefined,
  checkAuth: async () => undefined,
  checkError: async () => undefined,
  getPermissions: async () => "admin",
  canAccess: async () => true,
};

// Destination probe: renders the record seed that ra-core's useRecordFromLocation
// would read from `location.state.record` to pre-fill the Create form.
function CreateProbe() {
  const location = useLocation();
  const record = (location.state as { record?: Record<string, unknown> } | null)?.record;
  return (
    <div data-testid="create-probe" data-geofence-id={String(record?.geofence_id ?? "")} />
  );
}

describe("CreateButton state pre-fill", () => {
  it("forwards state.record so the Create form is seeded with the parent id", async () => {
    const screen = render(
      // AdminContext already provides a Router — nesting our own would throw
      // "Router inside Router"; define the probe routes under its router.
      <AdminContext dataProvider={testDataProvider()} authProvider={authProvider}>
        <Routes>
          <Route
            path="/"
            element={
              <ResourceContextProvider value="route">
                <CreateButton
                  resource="route"
                  label="New route"
                  state={{ record: { geofence_id: 7 } }}
                />
              </ResourceContextProvider>
            }
          />
          <Route path="/route/create" element={<CreateProbe />} />
        </Routes>
      </AdminContext>,
    );

    await screen.getByRole("link", { name: /new route/i }).click();
    await expect
      .element(screen.getByTestId("create-probe"))
      .toHaveAttribute("data-geofence-id", "7");
  });
});
