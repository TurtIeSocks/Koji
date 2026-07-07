import { describe, expect, it, vi } from "vitest";
import { render } from "vitest-browser-react";
import { AdminContext } from "@/components/admin";
import { Notification } from "@/components/admin/feedback/notification";
import {
  RecordContextProvider,
  ResourceContextProvider,
  testDataProvider,
} from "shadmin-core";
import type { AuthProvider } from "shadmin-core";

vi.mock("@/lib/http", async (orig) => {
  const actual = (await orig()) as object;
  return {
    ...actual,
    internalFetch: vi.fn(async () => ({
      status: 200,
      json: {
        status: "ok",
        data: { delivered: true, upstream_status: 200, error: null },
      },
    })),
  };
});

import { internalFetch } from "@/lib/http";
import { WebhookTestButton } from "@/resources/webhook/webhook-test-button";

const stubDataProvider = { ...testDataProvider(), subscribe: () => () => undefined };
const stubAuthProvider: AuthProvider = {
  login: async () => undefined,
  logout: async () => undefined,
  checkAuth: async () => undefined,
  checkError: async () => undefined,
  getPermissions: async () => "admin",
  canAccess: async () => true,
};

describe("WebhookTestButton", () => {
  it("fires POST /webhooks/:id/test and toasts the result", async () => {
    const screen = render(
      <AdminContext dataProvider={stubDataProvider} authProvider={stubAuthProvider}>
        <ResourceContextProvider value="webhook">
          <RecordContextProvider value={{ id: 7, name: "hook" }}>
            <WebhookTestButton />
          </RecordContextProvider>
        </ResourceContextProvider>
        <Notification />
      </AdminContext>,
    );
    await screen.getByRole("button", { name: /test/i }).click();
    expect(internalFetch).toHaveBeenCalledWith("/webhooks/7/test", { method: "POST" });
    await expect.element(screen.getByText(/Delivered/i)).toBeVisible();
  });
});
