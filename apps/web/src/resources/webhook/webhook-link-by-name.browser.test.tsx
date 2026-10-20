import { beforeEach, describe, expect, it, vi } from "vitest";
import { userEvent } from "@vitest/browser/context";
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
  return { ...actual, internalFetch: vi.fn() };
});

import { internalFetch } from "@/lib/http";
import { WebhookLinkByName } from "@/resources/webhook/webhook-link-by-name";

const stubDataProvider = { ...testDataProvider(), subscribe: () => () => undefined };
const stubAuthProvider: AuthProvider = {
  login: async () => undefined,
  logout: async () => undefined,
  checkAuth: async () => undefined,
  checkError: async () => undefined,
  getPermissions: async () => "admin",
  canAccess: async () => true,
};

const plan = (overrides: Record<string, unknown> = {}) => ({
  webhook_id: 8,
  dry_run: true,
  linked: [
    { geofence: 1, name: "Denver", mode: "pokemon", dragonite_area_id: 7 },
    { geofence: 2, name: "Denver", mode: "quest", dragonite_area_id: 7 },
  ],
  already_linked: [{ geofence: 3, name: "Boulder", mode: "fort", dragonite_area_id: 9 }],
  conflicts: [
    {
      geofence: 4,
      name: "Aurora",
      mode: "pokemon",
      dragonite_area_id: 3,
      wanted_dragonite_area_id: 11,
    },
  ],
  area_taken: [],
  areas_without_geofence: [
    { dragonite_area_id: 20, name: "Nowhere" },
    { dragonite_area_id: 21, name: "Elsewhere" },
    { dragonite_area_id: 22, name: "Anywhere" },
  ],
  geofences_without_area: [{ geofence: 5, name: "Stray", mode: "fort" }],
  ...overrides,
});

const answer = (data: unknown) => ({ status: 200, json: { status: "ok", data } });

const bodyOf = (call: number) =>
  JSON.parse(vi.mocked(internalFetch).mock.calls[call][1]!.body as string);

const renderButton = (record: Record<string, unknown> = { id: 8, mode: "dragonite" }) =>
  render(
    <AdminContext dataProvider={stubDataProvider} authProvider={stubAuthProvider}>
      <ResourceContextProvider value="webhook">
        <RecordContextProvider value={record}>
          <WebhookLinkByName />
        </RecordContextProvider>
      </ResourceContextProvider>
      <Notification />
    </AdminContext>,
  );

describe("WebhookLinkByName", () => {
  beforeEach(() => {
    vi.mocked(internalFetch).mockReset();
  });

  it("is offered on a Dragonite webhook only", async () => {
    const screen = renderButton({ id: 1, mode: "event" });
    await expect.element(screen.getByRole("button", { name: /link by name/i })).not.toBeInTheDocument();
  });

  it("previews the dry-run plan with a count on every section", async () => {
    vi.mocked(internalFetch).mockResolvedValueOnce(answer(plan()) as never);
    const screen = renderButton();
    await screen.getByRole("button", { name: /link by name/i }).click();

    await expect.element(screen.getByRole("button", { name: /^will link\s*2$/i })).toBeVisible();
    await expect.element(screen.getByRole("button", { name: /^already linked\s*1$/i })).toBeVisible();
    await expect.element(screen.getByRole("button", { name: /^conflicts\s*1$/i })).toBeVisible();
    await expect.element(screen.getByRole("button", { name: /^areas without a geofence\s*3$/i })).toBeVisible();
    await expect.element(screen.getByRole("button", { name: /^geofences without an area\s*1$/i })).toBeVisible();

    expect(internalFetch).toHaveBeenCalledTimes(1);
    expect(internalFetch).toHaveBeenCalledWith(
      "/geofences/link-dragonite",
      expect.objectContaining({ method: "POST" }),
    );
    expect(bodyOf(0)).toEqual({ webhook_id: 8, dry_run: true });
  });

  it("opens a section to list its rows", async () => {
    vi.mocked(internalFetch).mockResolvedValueOnce(answer(plan()) as never);
    const screen = renderButton();
    await screen.getByRole("button", { name: /link by name/i }).click();

    await expect.element(screen.getByText("Nowhere")).not.toBeInTheDocument();
    await screen.getByRole("button", { name: /^areas without a geofence/i }).click();
    await expect.element(screen.getByText("Nowhere")).toBeVisible();
    await expect.element(screen.getByText("Anywhere")).toBeVisible();
  });

  it("applies without dry_run and shows what was linked", async () => {
    vi.mocked(internalFetch)
      .mockResolvedValueOnce(answer(plan()) as never)
      .mockResolvedValueOnce(answer(plan({ dry_run: false })) as never);
    const screen = renderButton();
    await screen.getByRole("button", { name: /link by name/i }).click();
    await screen.getByRole("button", { name: /^apply$/i }).click();

    await expect.element(screen.getByText(/linked 2 geofences/i)).toBeVisible();
    expect(internalFetch).toHaveBeenCalledTimes(2);
    expect(bodyOf(1)).toEqual({ webhook_id: 8 });
    expect(bodyOf(1)).not.toHaveProperty("dry_run");
  });

  it("writes nothing on Cancel", async () => {
    vi.mocked(internalFetch).mockResolvedValueOnce(answer(plan()) as never);
    const screen = renderButton();
    await screen.getByRole("button", { name: /link by name/i }).click();
    await expect.element(screen.getByRole("button", { name: /^will link/i })).toBeVisible();
    await screen.getByRole("button", { name: /^cancel$/i }).click();

    await expect.element(screen.getByRole("dialog")).not.toBeInTheDocument();
    expect(internalFetch).toHaveBeenCalledTimes(1);
    expect(bodyOf(0).dry_run).toBe(true);
  });

  it("stays open while Apply runs and then shows what was linked", async () => {
    let finish!: (v: unknown) => void;
    vi.mocked(internalFetch)
      .mockResolvedValueOnce(answer(plan()) as never)
      .mockReturnValueOnce(new Promise((resolve) => (finish = resolve)) as never);
    const screen = renderButton();
    await screen.getByRole("button", { name: /link by name/i }).click();
    await screen.getByRole("button", { name: /^apply$/i }).click();
    await expect.element(screen.getByRole("button", { name: /applying/i })).toBeVisible();

    await userEvent.keyboard("{Escape}");
    await expect.element(screen.getByRole("dialog")).toBeVisible();
    await expect.element(screen.getByRole("button", { name: /^close$/i })).not.toBeInTheDocument();

    finish(answer(plan({ dry_run: false })));
    await expect.element(screen.getByText(/linked 2 geofences/i)).toBeVisible();
  });

  it("cannot apply a plan with nothing to link", async () => {
    vi.mocked(internalFetch).mockResolvedValueOnce(answer(plan({ linked: [] })) as never);
    const screen = renderButton();
    await screen.getByRole("button", { name: /link by name/i }).click();
    await expect.element(screen.getByRole("button", { name: /^will link\s*0$/i })).toBeVisible();
    await expect.element(screen.getByRole("button", { name: /^apply$/i })).toBeDisabled();
  });

  it("shows the server's message when the plan cannot be read", async () => {
    vi.mocked(internalFetch).mockResolvedValueOnce({
      status: 502,
      json: { status: "error", error: { message: "could not read Dragonite areas: refused" } },
    } as never);
    const screen = renderButton();
    await screen.getByRole("button", { name: /link by name/i }).click();
    await expect.element(screen.getByText(/could not read Dragonite areas/i)).toBeVisible();
    await expect.element(screen.getByRole("button", { name: /^apply$/i })).not.toBeInTheDocument();
  });

  it("stays responsive on a plan of thousands of rows", async () => {
    const many = Array.from({ length: 5000 }, (_, i) => ({
      geofence: i + 1,
      name: `Fence ${i + 1}`,
      mode: "pokemon",
      dragonite_area_id: i + 1,
    }));
    vi.mocked(internalFetch).mockResolvedValueOnce(
      answer(
        plan({
          linked: many,
          geofences_without_area: many.map(({ geofence, name, mode }) => ({ geofence, name, mode })),
        }),
      ) as never,
    );
    const screen = renderButton();
    const started = performance.now();
    await screen.getByRole("button", { name: /link by name/i }).click();
    await expect.element(screen.getByRole("button", { name: /^will link\s*5000$/i })).toBeVisible();
    await screen.getByRole("button", { name: /^will link/i }).click();
    await expect.element(screen.getByText(/Fence 1 \(pokemon\) to Area 1$/)).toBeVisible();

    // Only a first page is in the DOM; the rest waits behind "Show more".
    expect(document.querySelectorAll("[data-slot=link-row]").length).toBeLessThan(500);
    await expect.element(screen.getByRole("button", { name: /show more/i })).toBeVisible();
    expect(performance.now() - started).toBeLessThan(3000);
  });
});
