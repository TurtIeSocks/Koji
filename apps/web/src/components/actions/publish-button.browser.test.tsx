import { describe, expect, it, vi } from "vitest";
import { render } from "vitest-browser-react";
import { AdminContext } from "@/components/admin";
import {
  ResourceContextProvider,
  testDataProvider,
  ListContextProvider,
  RecordContextProvider,
} from "shadmin-core";
import type { AuthProvider } from "shadmin-core";
import { PublishButton, BulkPublishButton } from "./publish-button";

// Stub internalFetch — will be overridden per test
vi.mock("@/lib/http", () => ({
  internalFetch: vi
    .fn()
    .mockResolvedValue({ status: 200, json: { status: "ok", data: {} } }),
}));

// Spy useNotify — AdminContext alone doesn't mount the sonner <Toaster>, so the
// toast never hits the DOM; assert the notify CALL instead. importActual keeps
// every other shadmin-core export (contexts, providers) real.
const { notifyMock } = vi.hoisted(() => ({ notifyMock: vi.fn() }));
vi.mock("shadmin-core", async (importActual) => {
  const actual = await importActual<typeof import("shadmin-core")>();
  return { ...actual, useNotify: () => notifyMock };
});

const fakeRecord = { id: 7, name: "FenceAlpha", mode: "pokemon" };

const stubDataProvider = {
  ...testDataProvider({
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    getList: async () => ({ data: [] as any, total: 0 }),
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    getOne: async () => ({ data: fakeRecord as any }),
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

const wrapSingle = (node: React.ReactNode) => (
  <AdminContext dataProvider={stubDataProvider} authProvider={stubAuthProvider}>
    <ResourceContextProvider value="geofence">
      {/* eslint-disable-next-line @typescript-eslint/no-explicit-any */}
      <RecordContextProvider value={fakeRecord as any}>
        {node}
      </RecordContextProvider>
    </ResourceContextProvider>
  </AdminContext>
);

const wrapBulk = (node: React.ReactNode) => (
  <AdminContext dataProvider={stubDataProvider} authProvider={stubAuthProvider}>
    <ResourceContextProvider value="geofence">
      <ListContextProvider
        value={
          {
            selectedIds: [1, 2],
            onSelect: () => undefined,
            onToggleItem: () => undefined,
            onUnselectItems: () => undefined,
          } as any // eslint-disable-line @typescript-eslint/no-explicit-any
        }
      >
        {node}
      </ListContextProvider>
    </ResourceContextProvider>
  </AdminContext>
);

describe("PublishButton", () => {
  it("renders a Publish button", async () => {
    const screen = render(wrapSingle(<PublishButton />));
    await expect
      .element(screen.getByRole("button", { name: /publish/i }))
      .toBeVisible();
  });

  it("POSTs to /internal/geofences/{id}/publish on click", async () => {
    const { internalFetch } = await import("@/lib/http");
    const mockFetch = vi.mocked(internalFetch);
    mockFetch.mockResolvedValueOnce({ status: 200, json: { status: "ok", data: {} } });

    const screen = render(wrapSingle(<PublishButton />));
    await screen.getByRole("button", { name: /publish/i }).click();

    expect(mockFetch).toHaveBeenCalledWith(
      "/geofences/7/publish",
      expect.objectContaining({ method: "POST" }),
    );
  });

  it("shows warning notify on 422", async () => {
    const { internalFetch } = await import("@/lib/http");
    const mockFetch = vi.mocked(internalFetch);
    mockFetch.mockResolvedValueOnce({
      status: 422,
      json: { error: "No linked Dragonite area" },
    });

    notifyMock.mockClear();
    const screen = render(wrapSingle(<PublishButton />));
    await screen.getByRole("button", { name: /publish/i }).click();

    // 422 → a warning notification with the server's message (not a crash).
    await vi.waitFor(() =>
      expect(notifyMock).toHaveBeenCalledWith(
        expect.stringMatching(/no linked dragonite area/i),
        expect.objectContaining({ type: "warning" }),
      ),
    );
  });
});

describe("BulkPublishButton", () => {
  it("renders a Publish button in bulk context", async () => {
    const screen = render(wrapBulk(<BulkPublishButton />));
    await expect
      .element(screen.getByRole("button", { name: /publish/i }))
      .toBeVisible();
  });
});
