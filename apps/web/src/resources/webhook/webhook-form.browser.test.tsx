import { describe, expect, it } from "vitest";
import { render } from "vitest-browser-react";
import { AdminContext } from "@/components/admin";
import { ResourceContextProvider, testDataProvider } from "shadmin-core";
import type { AuthProvider } from "shadmin-core";
import { WebhookCreate } from "@/resources/webhook/webhook-create";

const stubDataProvider = {
  ...testDataProvider({
    getList: async () => ({ data: [{ id: 10, name: "Proj-A" }] as any, total: 1 }),
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

describe("WebhookCreate mode-conditional fields", () => {
  it("shows secret+topics for event, method for ping", async () => {
    const screen = render(
      <AdminContext dataProvider={stubDataProvider} authProvider={stubAuthProvider}>
        <ResourceContextProvider value="webhook">
          <WebhookCreate />
        </ResourceContextProvider>
      </AdminContext>,
    );
    // default mode = event → secret label visible, method label not
    await expect.element(screen.getByLabelText(/secret/i)).toBeVisible();
    // switch mode to ping: shadcn Select is not a native <select>, so drive it
    // by opening the trigger (labelled "Mode") then clicking the option text
    // (precedent: src/map/panels/filter-panel.browser.test.tsx).
    await screen.getByLabelText(/mode/i).click();
    await screen.getByRole("option", { name: "Ping (legacy reload)" }).click();
    await expect.element(screen.getByLabelText(/method/i)).toBeVisible();
  });
});
