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
  it("scopes markers to the fence bbox", async () => {
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
    // bbox of poly = [0,0,2,2]
    expect(useMarkers).toHaveBeenCalledWith("gym", [0, 0, 2, 2], expect.anything(), expect.anything());
  });
});
