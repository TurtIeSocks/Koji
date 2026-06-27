import { render } from "vitest-browser-react";
import { expect, test, vi } from "vitest";

// Mock the WebGL-bound libs so the smoke test runs headless.
vi.mock("react-map-gl/maplibre", () => ({
  Map: ({ children, onMove }: any) => (
    <div data-testid="maplibre" onClick={() => onMove?.({ viewState: { longitude: 1, latitude: 2, zoom: 5, pitch: 0, bearing: 0 } })}>
      {children}
    </div>
  ),
  useControl: () => ({}),
}));
vi.mock("@/map/data/use-markers", () => ({ useMarkers: () => ({ data: [] }) }));
vi.mock("@/map/data/use-geo-features", () => ({ useGeoFeatures: () => ({ data: { type: "FeatureCollection", features: [] } }) }));
vi.mock("@/map/data/use-s2-cells", () => ({ useS2Cells: () => ({ data: [] }) }));

import { DeckCanvas } from "@/map/deck-canvas";

test("DeckCanvas mounts the MapLibre root without throwing", async () => {
  const screen = render(<DeckCanvas />);
  await expect.element(screen.getByTestId("maplibre")).toBeInTheDocument();
});
