import { describe, expect, it } from "vitest";
import { render } from "vitest-browser-react";
import { AdminContext } from "@/components/admin";
import { testDataProvider } from "shadmin-core";
import type { AuthProvider } from "shadmin-core";
import { Dashboard } from "@/dashboard/dashboard";

const stubDataProvider = {
  ...testDataProvider({
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    getList: async () => ({ data: [] as any, total: 2 }),
  }),
  // Dashboard uses realtimeDataProvider shape — subscribe must exist
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

describe("Dashboard", () => {
  it("renders the geofence count card and the job queue panel", async () => {
    const screen = render(
      <AdminContext dataProvider={stubDataProvider} authProvider={stubAuthProvider}>
        <Dashboard />
      </AdminContext>,
    );
    await expect.element(screen.getByText(/geofences/i)).toBeVisible();
    await expect.element(screen.getByText(/job queue/i)).toBeVisible();
    // Count card resolves to the mock total (2).
    await expect.element(screen.getByText("2")).toBeVisible();
  });
});
