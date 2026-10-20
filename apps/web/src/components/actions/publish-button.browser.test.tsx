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

// Stub internalFetch — will be overridden per test. Spread the real module so
// the rest of `@api`'s live modules (apiV2Fetch/unwrapResponse consumers) stay
// resolvable in browser/ESM mode.
vi.mock("@/lib/http", async (importActual) => ({
  ...(await importActual<typeof import("@/lib/http")>()),
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

const wrapBulk = (node: React.ReactNode, selectedIds: number[] = [1, 2]) => (
  <AdminContext dataProvider={stubDataProvider} authProvider={stubAuthProvider}>
    <ResourceContextProvider value="geofence">
      <ListContextProvider
        value={
          {
            selectedIds,
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

/** What the stub server plans for a request: `writes` it would send and the
 *  geofences it `skipped`. A real (non-dry) request answers the writes whose
 *  geofence it names, like the server sending exactly the ids it is given. */
const serve = async (plan: { writes: Write[]; skipped?: Skip[] }) => {
  const { internalFetch } = await import("@/lib/http");
  const mockFetch = vi.mocked(internalFetch);
  mockFetch.mockReset();
  mockFetch.mockImplementation(async (_path, init) => {
    const body = JSON.parse(String(init?.body));
    const writes = body.dry_run
      ? plan.writes
      : plan.writes.filter((w) => body.ids.includes(w.geofence));
    return {
      status: body.dry_run ? 200 : 202,
      json: {
        status: "ok",
        data: { dry_run: body.dry_run, writes, skipped: plan.skipped ?? [] },
      },
    };
  });
  return mockFetch;
};

/** The request bodies sent to POST /geofences/publish, in order. */
const bodies = (mockFetch: ReturnType<typeof vi.mocked<typeof import("@/lib/http").internalFetch>>) =>
  mockFetch.mock.calls.map(([path, init]) => {
    expect(path).toBe("/geofences/publish");
    return JSON.parse(String(init?.body));
  });

interface Write {
  geofence: number;
  name: string;
  mode: string;
  webhook_id: number;
  webhook_name: string;
  dragonite_area_id: number;
  dragonite_area_name?: string | null;
}
interface Skip {
  geofence: number;
  name: string | null;
  mode: string | null;
  reason: "not_found" | "no_target" | "no_link";
  message: string;
}

const write = (geofence: number, webhook: number, area: number): Write => ({
  geofence,
  name: `Fence${geofence}`,
  mode: "pokemon",
  webhook_id: webhook,
  webhook_name: `dragonite-${webhook}`,
  dragonite_area_id: area,
  dragonite_area_name: `Area${area}`,
});

describe("PublishButton", () => {
  it("opens a dialog listing the write, and sends nothing yet", async () => {
    const mockFetch = await serve({ writes: [write(7, 1, 21)] });
    const screen = render(wrapSingle(<PublishButton />));
    await screen.getByRole("button", { name: /publish/i }).click();

    const dialog = screen.getByRole("dialog");
    await expect.element(dialog.getByText("Fence7")).toBeVisible();
    await expect.element(dialog.getByText("pokemon")).toBeVisible();
    await expect.element(dialog.getByText("dragonite-1 (id 1)")).toBeVisible();
    await expect.element(dialog.getByText(/Area21 \(id 21\)/)).toBeVisible();
    expect(bodies(mockFetch)).toEqual([{ ids: [7], dry_run: true }]);
  });

  it("lists a geofence that sends nothing apart, with its reason", async () => {
    await serve({
      writes: [],
      skipped: [
        {
          geofence: 7,
          name: "FenceAlpha",
          mode: "pokemon",
          reason: "no_target",
          message: "no active Dragonite webhook reaches geofence `FenceAlpha`",
        },
      ],
    });
    const screen = render(wrapSingle(<PublishButton />));
    await screen.getByRole("button", { name: /publish/i }).click();

    const dialog = screen.getByRole("dialog");
    await expect
      .element(dialog.getByText("No Dragonite target reaches these"))
      .toBeVisible();
    await expect.element(dialog.getByText("FenceAlpha")).toBeVisible();
    await expect
      .element(dialog.getByRole("button", { name: "Confirm" }))
      .toBeDisabled();
  });

  it("names the target that holds no link", async () => {
    await serve({
      writes: [],
      skipped: [
        {
          geofence: 7,
          name: "FenceAlpha",
          mode: "pokemon",
          reason: "no_link",
          message: "no link",
        },
      ],
    });
    const screen = render(wrapSingle(<PublishButton />));
    await screen.getByRole("button", { name: /publish/i }).click();

    await expect
      .element(
        screen
          .getByRole("dialog")
          .getByText("The target that reaches these holds no link"),
      )
      .toBeVisible();
  });

  it("Cancel sends nothing", async () => {
    const mockFetch = await serve({ writes: [write(7, 1, 21)] });
    const screen = render(wrapSingle(<PublishButton />));
    await screen.getByRole("button", { name: /publish/i }).click();
    await expect.element(screen.getByText("Fence7")).toBeVisible();

    await screen.getByRole("button", { name: "Cancel" }).click();

    await expect.element(screen.getByRole("dialog")).not.toBeInTheDocument();
    expect(bodies(mockFetch)).toEqual([{ ids: [7], dry_run: true }]);
  });

  it("Confirm sends the listed write", async () => {
    const mockFetch = await serve({ writes: [write(7, 1, 21)] });
    notifyMock.mockClear();
    const screen = render(wrapSingle(<PublishButton />));
    await screen.getByRole("button", { name: /publish/i }).click();
    await expect.element(screen.getByText("Fence7")).toBeVisible();

    await screen.getByRole("button", { name: "Confirm" }).click();

    await vi.waitFor(() =>
      expect(bodies(mockFetch)).toEqual([
        { ids: [7], dry_run: true },
        { ids: [7], dry_run: false },
      ]),
    );
    await vi.waitFor(() =>
      expect(notifyMock).toHaveBeenCalledWith(
        "1 write queued for 1 geofence",
        expect.objectContaining({ type: "info" }),
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

  it("groups the selection by target with counts, and Confirm sends only the geofences with a write", async () => {
    const mockFetch = await serve({
      writes: [write(1, 1, 11), write(2, 1, 12), write(2, 2, 22)],
      skipped: [
        {
          geofence: 3,
          name: "Fence3",
          mode: "quest",
          reason: "no_link",
          message: "no link",
        },
      ],
    });
    const screen = render(wrapBulk(<BulkPublishButton />, [1, 2, 3]));
    await screen.getByRole("button", { name: /publish/i }).click();

    const dialog = screen.getByRole("dialog");
    await expect
      .element(
        dialog.getByText(/3 writes to 2 Dragonite targets, 1 geofence skipped/),
      )
      .toBeVisible();
    await expect.element(dialog.getByText("dragonite-1 (id 1)")).toBeVisible();
    await expect.element(dialog.getByText("dragonite-2 (id 2)")).toBeVisible();
    await expect.element(dialog.getByText("Fence3")).toBeVisible();

    await dialog.getByRole("button", { name: "Confirm" }).click();

    await vi.waitFor(() =>
      expect(bodies(mockFetch)).toEqual([
        { ids: [1, 2, 3], dry_run: true },
        { ids: [1, 2], dry_run: false },
      ]),
    );
  });

  it("opens a 2,000-geofence selection without drawing every row", async () => {
    const ids = Array.from({ length: 2000 }, (_, i) => i + 1);
    await serve({ writes: ids.map((id) => write(id, (id % 2) + 1, id)) });
    const screen = render(wrapBulk(<BulkPublishButton />, ids));
    const started = performance.now();
    await screen.getByRole("button", { name: /publish/i }).click();

    const dialog = screen.getByRole("dialog");
    await expect
      .element(dialog.getByText(/2000 writes to 2 Dragonite targets/))
      .toBeVisible();
    // The counts are all there; the rows are drawn a page at a time.
    await expect.element(dialog.getByText("(1000)").first()).toBeVisible();
    expect(dialog.getByRole("listitem").elements().length).toBeLessThanOrEqual(
      200,
    );
    expect(performance.now() - started).toBeLessThan(5000);
  });
});
