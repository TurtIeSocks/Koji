import { describe, expect, it, vi } from "vitest";

vi.mock("@/map/data/use-markers", () => ({ useMarkers: vi.fn(() => ({ data: [] })) }));
vi.mock("@/map/data/use-s2-cells", () => ({ useS2Cells: vi.fn(() => ({ data: [] })) }));

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
});
