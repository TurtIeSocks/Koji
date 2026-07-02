import { render } from "vitest-browser-react";
import { MemoryRouter } from "react-router";
import { expect, test, vi } from "vitest";

vi.mock("@/map/deck-canvas", () => ({ DeckCanvas: () => <div data-testid="deck-canvas" /> }));
// DrawToolbar pulls in useDataProvider (no Admin context in this smoke test) — stub it.
vi.mock("@/map/panels/draw-toolbar", () => ({ DrawToolbar: () => <div data-testid="draw-toolbar" /> }));
// CalcPanel pulls in useQuery (no QueryClient here) — stub it too.
vi.mock("@/map/panels/calc-panel", () => ({ CalcPanel: () => <div data-testid="calc-panel" /> }));
import { MapRoute } from "@/map/map-route";

test("MapRoute renders the canvas, the layer drawer, and a back-to-admin link", async () => {
  // MapRoute holds a <Link> → needs a Router in the tree.
  const screen = render(
    <MemoryRouter>
      <MapRoute />
    </MemoryRouter>,
  );
  await expect.element(screen.getByTestId("deck-canvas")).toBeInTheDocument();
  await expect.element(screen.getByText(/Layers/)).toBeInTheDocument();
  await expect.element(screen.getByRole("link", { name: /Admin/ })).toBeInTheDocument();
});
