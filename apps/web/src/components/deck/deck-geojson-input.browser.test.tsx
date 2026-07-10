import { describe, expect, it } from "vitest";
import { render } from "vitest-browser-react";
import { AdminContext, SimpleForm } from "@/components/admin";
import { testDataProvider } from "shadmin-core";
import { DeckGeoJsonInput } from "./deck-geojson-input";

describe("DeckGeoJsonInput", () => {
  it("renders the edit map + draw toolbar inside a form", async () => {
    const screen = render(
      <AdminContext dataProvider={testDataProvider()}>
        <SimpleForm onSubmit={() => {}}>
          <DeckGeoJsonInput source="geometry" label="Geometry" />
        </SimpleForm>
      </AdminContext>,
    );
    await expect.element(screen.getByTestId("deck-map")).toBeInTheDocument();
    await expect.element(screen.getByRole("button", { name: /polygon/i })).toBeInTheDocument();
  });
});
