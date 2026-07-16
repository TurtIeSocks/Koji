// Tailwind's utility classes (position/z-index) are only compiled into a real
// stylesheet when this global CSS entrypoint is imported — the toggle
// button's `.click()` wouldn't land over the deck.gl canvas without it
// (established in deck-geojson-field.browser.test.tsx / geofence-show.browser.test.tsx).
import "@/index.css";
import type { ReactNode } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("@/map/data/use-markers", () => ({ useMarkers: vi.fn(() => ({ data: [] })) }));
vi.mock("@/map/data/use-s2-cells", () => ({ useS2Cells: vi.fn(() => ({ data: [] })) }));

// Hoisted so both the vi.mock factory and the tests can drive it.
const { useNeighborOverlayMock, setOnMock } = vi.hoisted(() => ({
  useNeighborOverlayMock: vi.fn(),
  setOnMock: vi.fn(),
}));
// Partial mock: keep the real `padBbox` (geofence-map.tsx still calls it
// directly for the padded-geometry branch of its fallback bbox — Task 9 added
// a camera-bounds branch and a start-center branch alongside it) while
// replacing `useNeighborOverlay` itself so the toggle/layers stay
// test-driven.
vi.mock("./use-neighbor-overlay", async (importOriginal) => {
  const actual = await importOriginal<typeof import("./use-neighbor-overlay")>();
  return { ...actual, useNeighborOverlay: useNeighborOverlayMock };
});

import { render } from "vitest-browser-react";
import { AdminContext, SimpleForm } from "@/components/admin";
import { RecordContextProvider, testDataProvider } from "shadmin-core";
import { useMarkers } from "@/map/data/use-markers";
import { GeofenceMap } from "./geofence-map";

const poly: GeoJSON.Polygon = {
  type: "Polygon",
  coordinates: [[[0, 0], [2, 0], [2, 2], [0, 2], [0, 0]]],
};

// Shared form wrapper — factored out of the four pre-existing inline
// `AdminContext`/`RecordContextProvider`/`SimpleForm` blocks below so the two
// new tests (and any future ones) don't repeat it. `record` defaults to an
// edit-style record with geometry; pass `{}` for a create-page render (no
// geometry, no id) — exactly the "form has NO geometry" scenario Task 9's
// fallback-bbox regression test needs.
function wrap(children: ReactNode, record: Record<string, unknown> = { id: 1, geometry: poly }) {
  return (
    <AdminContext dataProvider={testDataProvider()}>
      <RecordContextProvider value={record}>
        <SimpleForm onSubmit={() => {}} record={record}>
          {children}
        </SimpleForm>
      </RecordContextProvider>
    </AdminContext>
  );
}

describe("GeofenceMap", () => {
  beforeEach(() => {
    useNeighborOverlayMock.mockReset();
    setOnMock.mockReset();
    useNeighborOverlayMock.mockReturnValue({
      on: false,
      setOn: setOnMock,
      layers: [],
      getTooltip: () => null,
      label: "Neighbors",
    });
  });

  it("queries markers with the fence polygon as the area (+ its bbox as fallback)", async () => {
    const screen = render(
      <AdminContext dataProvider={testDataProvider()}>
        <RecordContextProvider value={{ id: 1, geometry: poly }}>
          <SimpleForm onSubmit={() => {}} record={{ id: 1, geometry: poly }}>
            <GeofenceMap />
          </SimpleForm>
        </RecordContextProvider>
      </AdminContext>,
    );
    await expect.element(screen.getByTestId("deck-map")).toBeInTheDocument();
    // Signature: (category, area, bounds, lastSeen, enabled). The actual polygon
    // rides as `area`; bbox of poly = [0,0,2,2] is the fallback.
    expect(useMarkers).toHaveBeenCalledWith("gym", poly, [0, 0, 2, 2], expect.anything(), expect.anything());
  });

  it("renders the last-seen date picker", async () => {
    const screen = render(
      <AdminContext dataProvider={testDataProvider()}>
        <RecordContextProvider value={{ id: 1, geometry: poly }}>
          <SimpleForm onSubmit={() => {}} record={{ id: 1, geometry: poly }}>
            <GeofenceMap />
          </SimpleForm>
        </RecordContextProvider>
      </AdminContext>,
    );
    await expect.element(screen.getByLabelText("Last seen after")).toBeInTheDocument();
  });

  it("shows a 'Show Neighbors' toggle in the control row and calls setOn on click", async () => {
    const screen = render(
      <AdminContext dataProvider={testDataProvider()}>
        <RecordContextProvider value={{ id: 1, geometry: poly }}>
          <SimpleForm onSubmit={() => {}} record={{ id: 1, geometry: poly }}>
            <GeofenceMap />
          </SimpleForm>
        </RecordContextProvider>
      </AdminContext>,
    );
    const toggle = screen.getByRole("button", { name: "Show Neighbors" });
    await expect.element(toggle).toBeInTheDocument();
    await toggle.click();
    expect(setOnMock).toHaveBeenCalledWith(true);
    // The map keeps rendering — no crash from wiring the overlay in.
    await expect.element(screen.getByTestId("deck-map")).toBeInTheDocument();
  });

  it("renders fine with neighbors off (back-compat: no crash, map still mounts)", async () => {
    const screen = render(
      <AdminContext dataProvider={testDataProvider()}>
        <RecordContextProvider value={{ id: 1, geometry: poly }}>
          <SimpleForm onSubmit={() => {}} record={{ id: 1, geometry: poly }}>
            <GeofenceMap />
          </SimpleForm>
        </RecordContextProvider>
      </AdminContext>,
    );
    await expect.element(screen.getByTestId("deck-map")).toBeInTheDocument();
    await expect.element(screen.getByRole("button", { name: "Show Neighbors" })).toBeInTheDocument();
  });

  it("renders the neighbour filter controls next to the toggle", async () => {
    const screen = render(wrap(<GeofenceMap />));
    await expect.element(screen.getByText("Show Neighbors")).toBeVisible();
    await expect.element(screen.getByLabelText("Neighbor mode filter")).toBeInTheDocument();
    await expect.element(screen.getByLabelText("My projects only")).toBeInTheDocument();
  });

  it("mode filter defaults to Auto and an override can return to it", async () => {
    const screen = render(wrap(<GeofenceMap />));
    const select = screen.getByLabelText("Neighbor mode filter");
    // Default = follow-form sentinel, surfaced as the Auto option.
    await expect.element(select).toHaveValue("auto");
    await select.selectOptions("pokemon");
    await expect.element(select).toHaveValue("pokemon");
    // The road back: picking Auto clears the override (regression — there was
    // no way to return to follow-form once an explicit mode was chosen).
    await select.selectOptions("auto");
    await expect.element(select).toHaveValue("auto");
  });

  it("neighbour fetch fires from the fallback bbox even with no geometry drawn", async () => {
    // Form has NO geometry (create page). Toggle on → a /geofences?…bbox=
    // request must still fire (start-center fallback) — regression for "Show
    // Neighbors does nothing".
    //
    // This file's `useNeighborOverlay` is a full mock of the hook (Task 8),
    // not the underlying fetch — there is no fetch stub to assert against
    // here. `use-neighbor-overlay.test.tsx`'s "threads filters into the fetch
    // URL" test hit the identical seam problem one level down (that file
    // mocks `useGeofencesByBbox` instead) and resolved it the same way:
    // assert on what the mocked hook was CALLED with rather than a URL
    // string. A non-null 4-number bbox arg here is what makes
    // `useGeofencesByBbox` build a `bbox=` query string inside the real
    // (unmocked in that lower test) hook — see use-geo-features.test.tsx for
    // the URL-building coverage itself.
    const screen = render(wrap(<GeofenceMap />, {}));
    const toggle = screen.getByRole("button", { name: "Show Neighbors" });
    await expect.element(toggle).toBeInTheDocument();
    await toggle.click();

    const [bbox] = useNeighborOverlayMock.mock.calls.at(-1)!;
    expect(bbox).not.toBeNull();
    expect(bbox).toHaveLength(4);
    expect((bbox as number[]).every((n) => Number.isFinite(n))).toBe(true);
  });
});
