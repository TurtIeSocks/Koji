import { render } from "vitest-browser-react";
import { expect, test, vi } from "vitest";

// Mock the WebGL-bound libs so the smoke test runs headless. deck.gl is the
// interaction root now; the MapLibre Map is its child.
vi.mock("@deck.gl/react", () => ({
  default: ({ children, onViewStateChange }: any) => (
    <div
      data-testid="deckgl"
      onClick={() => onViewStateChange?.({ viewState: { longitude: 1, latitude: 2, zoom: 5 } })}
    >
      {children}
    </div>
  ),
}));
vi.mock("react-map-gl/maplibre", () => ({
  Map: ({ children }: any) => <div data-testid="maplibre">{children}</div>,
}));
vi.mock("@/map/data/use-markers", () => ({ useMarkers: () => ({ data: [] }) }));
vi.mock("@/map/data/use-geo-features", () => ({ useGeoFeatures: () => ({ data: { type: "FeatureCollection", features: [] } }) }));
vi.mock("@/map/data/use-s2-cells", () => ({ useS2Cells: () => ({ data: [] }) }));
// Realtime needs a QueryClient + realtime context the smoke test doesn't mount.
vi.mock("@/map/data/use-map-realtime", () => ({ useMapRealtime: () => {} }));
vi.mock("@/map/data/use-calc-job", () => ({ useCalcJob: () => {} }));

import { DeckCanvas } from "@/map/deck-canvas";

test("DeckCanvas mounts the MapLibre root without throwing", async () => {
  const screen = render(<DeckCanvas />);
  await expect.element(screen.getByTestId("maplibre")).toBeInTheDocument();
});
