// Tailwind's utility classes (position/z-index) are only compiled into a real
// stylesheet when this global CSS entrypoint is imported — the marker toggle
// button's `.click()` wouldn't land over the deck.gl canvas without it
// (established in deck-geojson-field.browser.test.tsx).
import "@/index.css";
import { describe, expect, it, vi } from "vitest";
import { render } from "vitest-browser-react";
import { AdminContext } from "@/components/admin";
import { ResourceContextProvider, testDataProvider } from "shadmin-core";
import type { AuthProvider } from "shadmin-core";
import { GeofenceShow } from "@/resources/geofence/geofence-show";

// Real network fetch is disabled anyway (the toggle starts OFF, so
// useMarkerOverlay's `wantFetch` is false) — mocked regardless so no test in
// this file can accidentally hit the real golbat-data endpoint.
vi.mock("@/map/data/use-markers", () => ({
  useMarkers: vi.fn(() => ({ data: undefined })),
}));

// Hoisted so both the vi.mock factory and the test can see the same spy.
const { setOnMock } = vi.hoisted(() => ({ setOnMock: vi.fn() }));
// Partial mock: keep the real `padBbox` (geofence-show.tsx now calls it
// directly to build the hook's bbox arg) while replacing `useNeighborOverlay`
// itself so the toggle/layers stay test-driven.
vi.mock("@/components/deck/use-neighbor-overlay", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@/components/deck/use-neighbor-overlay")>();
  return {
    ...actual,
    useNeighborOverlay: vi.fn(() => ({
      on: false,
      setOn: setOnMock,
      layers: [],
      getTooltip: () => null,
      label: "Neighbors",
    })),
  };
});

const record = {
  id: 1,
  name: "Alpha",
  mode: "pokemon",
  geo_type: "Polygon",
  parent: null,
  projects: [20],
  geometry: { type: "Polygon", coordinates: [[[0, 0], [0, 1], [1, 1], [0, 0]]] },
};

// biome-ignore lint/suspicious/noExplicitAny: test-local capture of ra-core's getManyReference params
let captured: any;

const stubDataProvider = {
  ...testDataProvider({
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    getOne: async () => ({ data: record as any }),
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    getList: async () => ({ data: [] as any, total: 0 }),
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    getMany: async () => ({ data: [{ id: 20, name: "ProjectBeta" }] as any }),
    getManyReference: async (_resource, params) => {
      captured = params;
      return {
        data: [{ id: 5, name: "Route-A", mode: "circle_route", points: 12, geofence_id: 1 }] as any,
        total: 1,
      };
    },
  }),
  // subscribe() required by realtime context providers
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

describe("GeofenceShow", () => {
  it("renders the read-only geometry map and field values", async () => {
    const screen = render(
      <AdminContext dataProvider={stubDataProvider} authProvider={stubAuthProvider}>
        <ResourceContextProvider value="geofence">
          <GeofenceShow id={1} />
        </ResourceContextProvider>
      </AdminContext>,
    );
    await expect.element(screen.getByText("Alpha", { exact: true }).first()).toBeVisible();
    await expect.element(screen.getByTestId("deck-map")).toBeInTheDocument();
  });

  it("renders project chips in the projects field", async () => {
    const screen = render(
      <AdminContext dataProvider={stubDataProvider} authProvider={stubAuthProvider}>
        <ResourceContextProvider value="geofence">
          <GeofenceShow id={1} />
        </ResourceContextProvider>
      </AdminContext>,
    );
    await expect.element(screen.getByText("ProjectBeta")).toBeVisible();
  });

  it("lists routes scoped to the fence", async () => {
    const screen = render(
      <AdminContext dataProvider={stubDataProvider} authProvider={stubAuthProvider}>
        <ResourceContextProvider value="geofence">
          <GeofenceShow id={1} />
        </ResourceContextProvider>
      </AdminContext>,
    );
    await expect.element(screen.getByText("Route-A")).toBeVisible();
    // Guards the ra-core→dataProvider wiring: ReferenceManyField must pass
    // target="geofence_id" and id=1 through to getManyReference so the
    // dataProvider can translate them into a `?geofenceid=1` filter (see
    // data-provider.ts getManyReference TARGET_TO_PARAM override).
    expect(captured?.target).toBe("geofence_id");
    expect(captured?.id).toBe(1);
  });

  it("shows a mode-driven marker toggle + expand button on the map", async () => {
    const questRecord = { ...record, mode: "quest" };
    const questDataProvider = {
      ...testDataProvider({
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        getOne: async () => ({ data: questRecord as any }),
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        getList: async () => ({ data: [] as any, total: 0 }),
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        getMany: async () => ({ data: [] as any }),
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        getManyReference: async () => ({ data: [] as any, total: 0 }),
      }),
      subscribe: () => () => undefined,
    };

    const screen = render(
      <AdminContext dataProvider={questDataProvider} authProvider={stubAuthProvider}>
        <ResourceContextProvider value="geofence">
          <GeofenceShow id={1} />
        </ResourceContextProvider>
      </AdminContext>,
    );

    await expect
      .element(screen.getByRole("button", { name: "Show Pokestops" }))
      .toBeInTheDocument();
    await expect
      .element(screen.getByRole("button", { name: /expand/i }))
      .toBeInTheDocument();
  });

  it("shows a Show Neighbors toggle wired to useNeighborOverlay, and clicking it calls setOn", async () => {
    const questRecord = { ...record, mode: "quest" };
    const questDataProvider = {
      ...testDataProvider({
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        getOne: async () => ({ data: questRecord as any }),
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        getList: async () => ({ data: [] as any, total: 0 }),
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        getMany: async () => ({ data: [] as any }),
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        getManyReference: async () => ({ data: [] as any, total: 0 }),
      }),
      subscribe: () => () => undefined,
    };

    const screen = render(
      <AdminContext dataProvider={questDataProvider} authProvider={stubAuthProvider}>
        <ResourceContextProvider value="geofence">
          <GeofenceShow id={1} />
        </ResourceContextProvider>
      </AdminContext>,
    );

    const toggle = screen.getByRole("button", { name: "Show Neighbors" });
    await expect.element(toggle).toBeInTheDocument();

    await toggle.click();
    expect(setOnMock).toHaveBeenCalledWith(true);
  });
});
