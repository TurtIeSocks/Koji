// Tailwind's utility classes (position/z-index) are only compiled into a real
// stylesheet when this global CSS entrypoint is imported — the "browser"
// vitest project has no shared setupFiles, so a toolbar button's `.click()`
// wouldn't land over the deck.gl canvas without it (established in the
// DeckMap expand-button test).
import "@/index.css";
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

  it("shows Done/Cancel while drawing a polygon; Done disabled with <3 vertices", async () => {
    const screen = render(
      <AdminContext dataProvider={testDataProvider()}>
        <SimpleForm onSubmit={() => {}}>
          <DeckGeoJsonInput source="geometry" />
        </SimpleForm>
      </AdminContext>,
    );
    // Not drawing → no Done button.
    expect(screen.container.querySelector('[aria-label="Finish drawing"]')).toBeNull();
    await screen.getByRole("button", { name: "Polygon" }).click();
    await expect
      .element(screen.getByRole("button", { name: "Finish drawing" }))
      .toBeInTheDocument();
    await expect.element(screen.getByRole("button", { name: "Finish drawing" })).toBeDisabled();
    await expect
      .element(screen.getByRole("button", { name: "Cancel drawing" }))
      .toBeInTheDocument();
  });

  it("allowedModes restricts the toolbar buttons", async () => {
    const screen = render(
      <AdminContext dataProvider={testDataProvider()}>
        <SimpleForm onSubmit={() => {}}>
          <DeckGeoJsonInput source="geometry" allowedModes={["drawPolygon", "modify"]} />
        </SimpleForm>
      </AdminContext>,
    );
    await expect.element(screen.getByRole("button", { name: /polygon/i })).toBeInTheDocument();
    await expect.element(screen.getByRole("button", { name: /modify/i })).toBeInTheDocument();
    expect(screen.container.querySelector('[aria-label="Move"]')).toBeNull();
    expect(screen.container.querySelector('[aria-label="Split"]')).toBeNull();
  });
});
