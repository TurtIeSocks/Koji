import { describe, expect, it, vi } from "vitest";
import { render } from "vitest-browser-react";
import { AdminContext } from "@/components/admin";
import { ResourceContextProvider, testDataProvider } from "shadmin-core";
import type { AuthProvider } from "shadmin-core";
import {
  realtimeDataProvider,
  fakeTransport,
  inMemoryLockProvider,
  resourceTopic,
} from "@/components/realtime";
import { GeofenceList } from "@/resources/geofence/geofence-list";

const fakeRows = [
  { id: 1, name: "Alpha", mode: "pokemon", parent: null, geo_type: "Polygon" },
  { id: 2, name: "Beta", mode: "quest", parent: 1, geo_type: "MultiPolygon" },
];

const stubAuthProvider: AuthProvider = {
  login: async () => undefined,
  logout: async () => undefined,
  checkAuth: async () => undefined,
  checkError: async () => undefined,
  getPermissions: async () => "admin",
  canAccess: async () => true,
};

describe("geofence realtime", () => {
  it("refetches the list when a resource/geofence event arrives", async () => {
    const base = testDataProvider({
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      getList: async () => ({ data: fakeRows as any, total: fakeRows.length }),
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      getMany: async () => ({ data: [] as any }),
    });

    const getListSpy = vi.spyOn(base, "getList");
    const transport = fakeTransport();
    const dp = realtimeDataProvider(base, transport, {
      locks: inMemoryLockProvider(),
    });

    const screen = render(
      <AdminContext dataProvider={dp} authProvider={stubAuthProvider}>
        <ResourceContextProvider value="geofence">
          <GeofenceList />
        </ResourceContextProvider>
      </AdminContext>,
    );

    // Wait for initial render — confirms getList resolved + rows painted
    await expect.element(screen.getByText("Alpha")).toBeVisible();
    const callsBefore = getListSpy.mock.calls.length;

    // Server emits a resource/geofence created event via the fake transport.
    // This dispatches synchronously to the subscriber registered by
    // useSubscribeToRecordList → useGetListLive → invalidateQueries → refetch.
    await transport.publish(resourceTopic("geofence"), {
      type: "created",
      payload: { ids: [99] },
    });

    // Assert a second getList call arrives (invalidation → React Query refetch)
    await vi.waitFor(() => {
      expect(getListSpy.mock.calls.length).toBeGreaterThan(callsBefore);
    });

    getListSpy.mockRestore();
  });
});
