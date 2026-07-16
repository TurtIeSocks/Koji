// Tailwind's utility classes (position/z-index) are only compiled into a real
// stylesheet when this global CSS entrypoint is imported — the toggle
// button's `.click()` wouldn't land over the deck.gl canvas without it
// (established in deck-geojson-field.browser.test.tsx / geofence-show.browser.test.tsx).
import "@/index.css";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("@/map/data/use-markers", () => ({ useMarkers: vi.fn(() => ({ data: [] })) }));
vi.mock("@/map/data/use-s2-cells", () => ({ useS2Cells: vi.fn(() => ({ data: [] })) }));

// Hoisted so both the vi.mock factory and the tests can drive it.
const { useNeighborOverlayMock, setOnMock } = vi.hoisted(() => ({
  useNeighborOverlayMock: vi.fn(),
  setOnMock: vi.fn(),
}));
// Partial mock: keep the real `padBbox` (geofence-map.tsx now calls it
// directly to build the hook's bbox arg) while replacing `useNeighborOverlay`
// itself so the toggle/layers stay test-driven.
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
});
