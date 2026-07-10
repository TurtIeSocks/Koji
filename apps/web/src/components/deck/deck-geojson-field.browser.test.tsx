import { describe, expect, it } from "vitest";
import { render } from "vitest-browser-react";
import { RecordContextProvider } from "shadmin-core";
import { DeckGeoJsonField } from "./deck-geojson-field";

const poly: GeoJSON.Polygon = { type: "Polygon", coordinates: [[[0,0],[1,0],[1,1],[0,1],[0,0]]] };

describe("DeckGeoJsonField", () => {
  it("renders the map when the record has geometry", async () => {
    const screen = render(
      <RecordContextProvider value={{ id: 1, geometry: poly }}>
        <DeckGeoJsonField source="geometry" />
      </RecordContextProvider>,
    );
    await expect.element(screen.getByTestId("deck-map")).toBeInTheDocument();
  });
  it("shows empty text when geometry is missing", async () => {
    const screen = render(
      <RecordContextProvider value={{ id: 1 }}>
        <DeckGeoJsonField source="geometry" emptyText="No geometry" />
      </RecordContextProvider>,
    );
    await expect.element(screen.getByText("No geometry")).toBeVisible();
  });
});
