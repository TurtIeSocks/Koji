import { render } from "vitest-browser-react";
import { beforeEach, expect, test } from "vitest";
import { CoordinateReadout } from "@/map/panels/coordinate-readout";
import { useMapViewStore } from "@/map/stores/map-view-store";

beforeEach(() =>
  useMapViewStore.setState({
    settledViewState: { longitude: -122.33, latitude: 47.6, zoom: 12, pitch: 0, bearing: 0 },
  }),
);

test("readout shows the settled lng/lat/zoom", async () => {
  const screen = render(<CoordinateReadout />);
  await expect.element(screen.getByText(/-122.33/)).toBeInTheDocument();
  await expect.element(screen.getByText(/47.6/)).toBeInTheDocument();
});
