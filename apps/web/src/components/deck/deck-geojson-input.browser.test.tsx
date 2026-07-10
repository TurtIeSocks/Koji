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

  it("exposes the full toolset, with split/cut-hole gated until a shape is selected", async () => {
    const screen = render(
      <AdminContext dataProvider={testDataProvider()}>
        <SimpleForm onSubmit={() => {}}>
          <DeckGeoJsonInput source="geometry" label="Geometry" />
        </SimpleForm>
      </AdminContext>,
    );
    for (const name of [/polygon/i, /rectangle/i, /circle/i, /modify/i, /move/i, /split/i, /cut hole/i]) {
      await expect.element(screen.getByRole("button", { name })).toBeInTheDocument();
    }
    // No geometry here → nothing selected → the shape-ops that draw onto a
    // selection are disabled.
    await expect.element(screen.getByRole("button", { name: /split/i })).toBeDisabled();
    await expect.element(screen.getByRole("button", { name: /cut hole/i })).toBeDisabled();
  });

  it("draw-toolbar buttons are type=button so a click never submits the form", async () => {
    // Regression: shadcn <Button> spreads props with no default `type`, so inside
    // a SimpleForm a toolbar click defaulted to type="submit" → submitted the form
    // and navigated away. The buttons must carry type="button". (Asserting the
    // attribute rather than clicking: deck's canvas wrapper intercepts synthetic
    // pointer events in headless, though the toolbar is clickable in a real tab.)
    const screen = render(
      <AdminContext dataProvider={testDataProvider()}>
        <SimpleForm onSubmit={() => {}}>
          <DeckGeoJsonInput source="geometry" label="Geometry" />
        </SimpleForm>
      </AdminContext>,
    );
    await expect
      .element(screen.getByRole("button", { name: /polygon/i }))
      .toHaveAttribute("type", "button");
  });
});
