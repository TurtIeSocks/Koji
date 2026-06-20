import { describe, expect, it } from "vitest";
import { render } from "vitest-browser-react";
import { AdminContext } from "@/components/admin";
import { testDataProvider } from "shadmin-core";
import type { AuthProvider } from "shadmin-core";
import { ImportWizard } from "./import-wizard";

const auth: AuthProvider = {
  login: async () => undefined,
  logout: async () => undefined,
  checkAuth: async () => undefined,
  checkError: async () => undefined,
  getPermissions: async () => "admin",
  canAccess: async () => true,
};

const wrap = (node: React.ReactNode) => (
  <AdminContext dataProvider={testDataProvider({})} authProvider={auth}>
    {node}
  </AdminContext>
);

describe("ImportWizard shell", () => {
  it("renders the stepper starting on Source", async () => {
    const screen = render(wrap(<ImportWizard />));
    await expect.element(screen.getByText("Source")).toBeVisible();
    await expect.element(screen.getByText("Review")).toBeVisible();
    // Step 0 panel = the Source step (its paste field is present).
    await expect.element(screen.getByLabelText(/paste geojson/i)).toBeVisible();
  });
});
