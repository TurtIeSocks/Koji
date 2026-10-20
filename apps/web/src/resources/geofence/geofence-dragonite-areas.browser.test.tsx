import "@/index.css";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { render } from "vitest-browser-react";
import { MemoryRouter, Routes, Route } from "react-router";
import { AdminContext } from "@/components/admin";
import { Notification } from "@/components/admin/feedback/notification";
import { ResourceContextProvider, testDataProvider } from "shadmin-core";
import type { AuthProvider } from "shadmin-core";

// The Areas a Dragonite lists, by webhook id, as GET /webhooks/{id}/areas answers.
const areasByWebhook: Record<number, unknown[]> = {};

vi.mock("@/lib/http", async (orig) => {
  const actual = (await orig()) as object;
  return {
    ...actual,
    internalFetch: vi.fn(async (path: string) => {
      const id = Number(/\/webhooks\/(\d+)\/areas/.exec(path)?.[1]);
      return { status: 200, json: { status: "ok", data: areasByWebhook[id] ?? [] } };
    }),
  };
});

// The deck stack is not under test here.
vi.mock("@/components/deck", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@/components/deck")>();
  return { ...actual, GeofenceMap: () => <div>map</div> };
});

import { GeofenceCreate } from "@/resources/geofence/geofence-create";
import { GeofenceEdit } from "@/resources/geofence/geofence-edit";

const target = (id: number, name: string, over: Record<string, unknown> = {}) => ({
  id,
  name,
  mode: "dragonite",
  active: true,
  project_id: null,
  ...over,
});

const area = (id: number, name: string, held_by: unknown[] = []) => ({ id, name, held_by });

const TARGETS = [
  target(1, "East"),
  target(2, "West", { project_id: 10 }),
  target(3, "Pinger", { mode: "ping" }),
  target(4, "Retired", { active: false }),
];

const record = (over: Record<string, unknown> = {}) => ({
  id: 5,
  name: "Denver",
  mode: "pokemon",
  parent: null,
  projects: [],
  dragonite_links: [],
  geometry: { type: "Polygon", coordinates: [[[0, 0], [1, 0], [1, 1], [0, 1], [0, 0]]] },
  ...over,
});

let current = record();
const update = vi.fn(async (_resource: string, params: { data: object }) => ({
  data: { ...params.data, id: 5 },
}));
const create = vi.fn(async (_resource: string, params: { data: object }) => ({
  data: { ...params.data, id: 6 },
}));

const stubDataProvider = () => ({
  ...testDataProvider({
    getList: async (resource: string) => {
      const data =
        resource === "webhook"
          ? TARGETS
          : resource === "project"
            ? [{ id: 10, name: "ProjectAlpha" }]
            : [];
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      return { data: data as any, total: data.length };
    },
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    getMany: async () => ({ data: [{ id: 10, name: "ProjectAlpha" }] as any }),
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    getOne: async () => ({ data: current as any }),
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    update: update as any,
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    create: create as any,
  }),
  subscribe: () => () => undefined,
});

const stubAuthProvider: AuthProvider = {
  login: async () => undefined,
  logout: async () => undefined,
  checkAuth: async () => undefined,
  checkError: async () => undefined,
  getPermissions: async () => "admin",
  canAccess: async () => true,
};

const wrap = (node: React.ReactNode) => (
  <MemoryRouter>
    <Routes>
      <Route
        path="/*"
        element={
          <AdminContext dataProvider={stubDataProvider()} authProvider={stubAuthProvider}>
            <ResourceContextProvider value="geofence">{node}</ResourceContextProvider>
            <Notification />
          </AdminContext>
        }
      />
    </Routes>
  </MemoryRouter>
);

const renderEdit = () => render(wrap(<GeofenceEdit id={5} mutationMode="pessimistic" />));

beforeEach(() => {
  update.mockClear();
  create.mockClear();
  current = record();
  areasByWebhook[1] = [area(1, "Boulder"), area(2, "denver"), area(3, "Aspen")];
  areasByWebhook[2] = [area(9, "Vail")];
});

describe("Geofence Dragonite Area picker", () => {
  it("shows one picker per active Dragonite target that reaches the geofence", async () => {
    const screen = renderEdit();
    await expect
      .element(screen.getByRole("combobox", { name: "Dragonite Area: East" }))
      .toBeVisible();
    // West is scoped to a project the geofence is not in; the ping and the
    // inactive row are not Dragonite targets that can reach anything.
    await expect
      .element(screen.getByRole("combobox", { name: "Dragonite Area: West" }))
      .not.toBeInTheDocument();
    await expect.element(screen.getByText("Pinger")).not.toBeInTheDocument();
    await expect.element(screen.getByText("Retired")).not.toBeInTheDocument();
  });

  it("puts the Area named like the geofence, ignoring case, alone in a Suggested section", async () => {
    const screen = renderEdit();
    await screen.getByRole("combobox", { name: "Dragonite Area: East" }).click();

    await expect.element(screen.getByText("Suggested", { exact: true })).toBeVisible();
    // Listed once: on top, not repeated below.
    expect(screen.getByRole("option", { name: "denver" }).elements()).toHaveLength(1);
    const options = screen.getByRole("option").elements();
    expect(options.map((o) => o.textContent)).toEqual(["denver", "Aspen", "Boulder"]);
  });

  it("has no Suggested section when no Area carries the geofence's name", async () => {
    current = record({ name: "Nowhere" });
    const screen = renderEdit();
    await screen.getByRole("combobox", { name: "Dragonite Area: East" }).click();

    await expect.element(screen.getByRole("option", { name: "Aspen" })).toBeVisible();
    await expect.element(screen.getByText("Suggested", { exact: true })).not.toBeInTheDocument();
  });

  it("lists the options A to Z", async () => {
    current = record({ name: "Nowhere" });
    areasByWebhook[1] = [area(1, "boulder"), area(2, "Vail"), area(3, "Aspen"), area(4, "avon")];
    const screen = renderEdit();
    await screen.getByRole("combobox", { name: "Dragonite Area: East" }).click();

    await expect.element(screen.getByRole("option", { name: "Aspen" })).toBeVisible();
    const options = screen.getByRole("option").elements();
    expect(options.map((o) => o.textContent)).toEqual(["Aspen", "avon", "boulder", "Vail"]);
  });

  it("sets the link through save", async () => {
    const screen = renderEdit();
    await screen.getByRole("combobox", { name: "Dragonite Area: East" }).click();
    await screen.getByRole("option", { name: "Boulder" }).click();
    await expect
      .element(screen.getByRole("combobox", { name: "Dragonite Area: East" }))
      .toHaveTextContent("Boulder");
    await screen.getByRole("button", { name: "Save" }).click();

    await vi.waitFor(() => expect(update).toHaveBeenCalled());
    expect(update.mock.calls[0][1].data).toMatchObject({
      dragonite_links: [{ webhook_id: 1, dragonite_area_id: 1 }],
    });
  });

  it("clears the link through save", async () => {
    current = record({ dragonite_links: [{ webhook_id: 1, dragonite_area_id: 3 }] });
    const screen = renderEdit();
    await expect
      .element(screen.getByRole("combobox", { name: "Dragonite Area: East" }))
      .toHaveTextContent("Aspen");
    await screen.getByRole("button", { name: "Clear Area for East" }).click();
    await expect
      .element(screen.getByRole("combobox", { name: "Dragonite Area: East" }))
      .toHaveTextContent("No Area");
    await screen.getByRole("button", { name: "Save" }).click();

    await vi.waitFor(() => expect(update).toHaveBeenCalled());
    expect(update.mock.calls[0][1].data).toMatchObject({
      dragonite_links: [{ webhook_id: 1, dragonite_area_id: null }],
    });
  });

  it("shows which geofence holds an Area and refuses to pick it, naming the holder", async () => {
    areasByWebhook[1] = [
      area(1, "Boulder", [{ geofence: 8, name: "Boulder quest fence", mode: "pokemon" }]),
      // Held in another mode: free for this geofence.
      area(2, "Aspen", [{ geofence: 9, name: "Aspen quest fence", mode: "quest" }]),
    ];
    const screen = renderEdit();
    await screen.getByRole("combobox", { name: "Dragonite Area: East" }).click();

    await expect
      .element(screen.getByText("held by Boulder quest fence", { exact: true }))
      .toBeVisible();
    await expect.element(screen.getByText(/held by Aspen/)).not.toBeInTheDocument();

    await screen.getByRole("option", { name: /Boulder/ }).click();
    await expect
      .element(screen.getByText(/already linked to "Boulder quest fence"/))
      .toBeVisible();
    await expect
      .element(screen.getByRole("combobox", { name: "Dragonite Area: East" }))
      .toHaveTextContent("No Area");
  });
});

describe("Geofence Dragonite Area picker on create", () => {
  it("shows a project-scoped target's picker once its project is chosen", async () => {
    const screen = render(wrap(<GeofenceCreate />));
    // East covers every geofence; West only the geofences of ProjectAlpha.
    await expect
      .element(screen.getByRole("combobox", { name: "Dragonite Area: East" }))
      .toBeVisible();
    await expect
      .element(screen.getByRole("combobox", { name: "Dragonite Area: West" }))
      .not.toBeInTheDocument();

    await screen.getByLabelText("Projects", { exact: true }).click();
    await screen.getByRole("option", { name: "ProjectAlpha" }).click();

    await expect
      .element(screen.getByRole("combobox", { name: "Dragonite Area: West" }))
      .toBeVisible();
  });

  it("drops a project-scoped target's link when its project is removed", async () => {
    const screen = render(wrap(<GeofenceCreate />));
    await screen.getByLabelText("Projects", { exact: true }).click();
    await screen.getByRole("option", { name: "ProjectAlpha" }).click();
    await screen.getByRole("combobox", { name: "Dragonite Area: West" }).click();
    await screen.getByRole("option", { name: "Vail" }).click();
    await expect
      .element(screen.getByRole("combobox", { name: "Dragonite Area: West" }))
      .toHaveTextContent("Vail");

    await screen.getByLabelText("Projects", { exact: true }).click();
    await screen.getByRole("option", { name: "ProjectAlpha" }).click();
    await expect
      .element(screen.getByRole("combobox", { name: "Dragonite Area: West" }))
      .not.toBeInTheDocument();
    await screen.getByLabelText(/name/i).fill("Fresh fence");
    await screen.getByRole("button", { name: "Save" }).click();

    await vi.waitFor(() => expect(create).toHaveBeenCalled());
    const links = (create.mock.calls[0][1].data as { dragonite_links?: unknown[] })
      .dragonite_links;
    expect(links ?? []).not.toContainEqual(expect.objectContaining({ webhook_id: 2 }));
  });
});
