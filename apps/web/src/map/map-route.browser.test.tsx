import { render } from "vitest-browser-react";
import { expect, test, vi } from "vitest";

vi.mock("@/map/deck-canvas", () => ({ DeckCanvas: () => <div data-testid="deck-canvas" /> }));
import { MapRoute } from "@/map/map-route";

test("MapRoute renders the canvas and the layer drawer together", async () => {
  const screen = render(<MapRoute />);
  await expect.element(screen.getByTestId("deck-canvas")).toBeInTheDocument();
  await expect.element(screen.getByText(/Layers/)).toBeInTheDocument();
});
