import { describe, expect, it, vi } from "vitest";
import { render } from "vitest-browser-react";
import { MemoryRouter } from "react-router";
import type { PickingInfo } from "@deck.gl/core";

const fence: GeoJSON.Feature = {
  type: "Feature",
  id: 42,
  properties: {},
  geometry: { type: "Polygon", coordinates: [[[0, 0], [1, 0], [1, 1], [0, 1], [0, 0]]] },
};

vi.mock("@/map/data/use-geo-features", () => ({
  useGeoFeatures: vi.fn(() => ({ data: { type: "FeatureCollection", features: [fence] } })),
}));

const navigateMock = vi.fn();
vi.mock("react-router", async (importOriginal) => {
  const actual = await importOriginal<typeof import("react-router")>();
  return { ...actual, useNavigate: () => navigateMock };
});

// Capture the onClick handler passed to GeoJsonLayer by subclassing the real
// layer — DeckGL still gets a legit deck.gl Layer instance, so rendering
// isn't disturbed, but we can invoke the click handler directly instead of
// simulating real WebGL canvas picking (not feasible in a browser test).
let capturedOnClick: ((info: PickingInfo) => void) | undefined;
vi.mock("@deck.gl/layers", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@deck.gl/layers")>();
  class SpyGeoJsonLayer extends actual.GeoJsonLayer {
    static override layerName = "SpyGeoJsonLayer";
    constructor(props: ConstructorParameters<typeof actual.GeoJsonLayer>[0]) {
      super(props);
      capturedOnClick = (props as { onClick?: (info: PickingInfo) => void }).onClick;
    }
  }
  return { ...actual, GeoJsonLayer: SpyGeoJsonLayer };
});

import { MapIndex } from "./map-index";

describe("MapIndex", () => {
  it("renders a full-bleed read-only map of all geofences", async () => {
    const screen = render(
      <MemoryRouter>
        <MapIndex />
      </MemoryRouter>,
    );
    await expect.element(screen.getByTestId("deck-map")).toBeInTheDocument();
  });

  it("navigates to the fence's page on click", async () => {
    render(
      <MemoryRouter>
        <MapIndex />
      </MemoryRouter>,
    );
    expect(capturedOnClick).toBeTypeOf("function");
    capturedOnClick?.({ object: fence } as PickingInfo);
    expect(navigateMock).toHaveBeenCalledWith("/geofence/42");
  });
});
