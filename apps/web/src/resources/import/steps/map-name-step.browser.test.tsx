import { describe, expect, it } from "vitest";
import { render } from "vitest-browser-react";
import { FormProvider, useForm } from "react-hook-form";
import { testDataProvider } from "shadmin-core";
import { AdminContext } from "@/components/admin";
import { MapNameStep } from "./map-name-step";

interface Feature {
  type: "Feature";
  geometry: unknown;
  properties: Record<string, unknown> | null;
}

const makeFeature = (props: Record<string, unknown>): Feature => ({
  type: "Feature",
  geometry: { type: "Polygon", coordinates: [[[0, 0], [0, 1], [1, 1], [0, 0]]] },
  properties: props,
});

// DeckGeoJsonField now calls useMarkerOverlay unconditionally (even with no
// markerMode), which always calls useMarkers -> react-query's useQuery — so
// this harness needs a QueryClientProvider in its tree just like production
// (App.tsx renders MapNameStep's ImportWizard inside <Admin>).
const Harness = ({
  features,
  sourceLoaded,
}: {
  features: Feature[];
  sourceLoaded?: boolean;
}) => {
  const methods = useForm({
    defaultValues: { features: features as unknown[], _source_loaded: sourceLoaded ?? false },
  });
  return (
    <AdminContext dataProvider={testDataProvider()}>
      <FormProvider {...methods}>
        <MapNameStep />
      </FormProvider>
    </AdminContext>
  );
};

describe("MapNameStep", () => {
  it("renders the map preview container", async () => {
    const features = [makeFeature({ title: "Zone A" })];
    const screen = render(<Harness features={features} />);
    await expect
      .element(screen.getByTestId("deck-map"))
      .toBeInTheDocument();
  });

  it("shows a property key from seeded features in the name-property select", async () => {
    const features = [
      makeFeature({ title: "Zone A" }),
      makeFeature({ title: "Zone B" }),
    ];
    const screen = render(<Harness features={features} />);
    // The select should have an option for the "title" property
    await expect.element(screen.getByRole("option", { name: "title" })).toBeInTheDocument();
  });

  it("shows the empty-state box before any source was loaded", async () => {
    const screen = render(<Harness features={[]} />);
    await expect.element(screen.getByText("Load features first")).toBeVisible();
    expect(screen.container.querySelector('[data-testid="deck-map"]')).toBeNull();
  });

  it("keeps the editable map mounted after deleting the last shape (regression)", async () => {
    // _source_loaded=true + empty features = the user loaded a source, then
    // deleted every shape on the map. The editor must stay mounted so they can
    // draw a replacement without navigating back to Source.
    const screen = render(<Harness features={[]} sourceLoaded />);
    await expect.element(screen.getByTestId("deck-map")).toBeInTheDocument();
    expect(screen.container.textContent).not.toContain("Load features first");
  });

  it("shows a duplicate-name warning when two features resolve to the same name", async () => {
    const features = [
      makeFeature({ title: "Same" }),
      makeFeature({ title: "Same" }),
    ];
    const screen = render(<Harness features={features} />);
    // Select "title" as the name property
    const select = screen.getByLabelText(/name property/i);
    await select.selectOptions("title");
    // With default template "{name}", both resolve to "Same" → duplicate warning
    await expect
      .element(screen.getByText(/duplicate/i))
      .toBeVisible();
  });
});
