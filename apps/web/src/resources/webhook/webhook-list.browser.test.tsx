import { describe, expect, it } from "vitest";
import { render } from "vitest-browser-react";
import { AdminContext } from "@/components/admin";
import { ResourceContextProvider, testDataProvider } from "shadmin-core";
import type { AuthProvider } from "shadmin-core";
import { WebhookList } from "@/resources/webhook/webhook-list";

const fakeRows = [
  { id: 1, name: "ReactMap reload", url: "http://rm/reload", mode: "ping", active: true, project_id: 10 },
  { id: 2, name: "Global events", url: "http://ev/hook", mode: "event", active: true, project_id: null },
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

describe("WebhookList", () => {
  it("renders webhook rows by name", async () => {
    const screen = render(
      <AdminContext dataProvider={stubDataProvider} authProvider={stubAuthProvider}>
        <ResourceContextProvider value="webhook">
          <WebhookList />
        </ResourceContextProvider>
      </AdminContext>,
    );
    await expect.element(screen.getByText("ReactMap reload")).toBeVisible();
    await expect.element(screen.getByText("Global events")).toBeVisible();
  });
});
