import { describe, expect, it, vi } from "vitest";
import { render } from "vitest-browser-react";
import { MemoryRouter } from "react-router";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";

vi.mock("@/map/data/use-markers", () => ({ useMarkers: vi.fn(() => ({ data: [] })) }));
vi.mock("@/map/data/calc-client", () => ({
  getAlgorithms: vi.fn().mockResolvedValue({ clustering: [], routing: [], bootstrap: [] }),
  submitCalc: vi.fn(),
  getJob: vi.fn(),
}));
vi.mock("@/components/realtime", () => ({ useSubscribe: vi.fn() }));

import { MapPlayground } from "./map-playground";

describe("MapPlayground", () => {
  it("renders the draw tools, a data-mode toggle, and a calc panel disabled until you draw", async () => {
    const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    const screen = render(
      <QueryClientProvider client={qc}>
        <MemoryRouter>
          <MapPlayground />
        </MemoryRouter>
      </QueryClientProvider>,
    );
    await expect.element(screen.getByTestId("deck-map")).toBeInTheDocument();
    // geofence draw tools
    await expect.element(screen.getByRole("button", { name: /polygon/i })).toBeInTheDocument();
    // data-mode toggle
    await expect.element(screen.getByRole("button", { name: /quest/i })).toBeInTheDocument();
    // calc panel present, disabled until an area is drawn
    await expect.element(screen.getByRole("button", { name: /calculate/i })).toBeDisabled();
    // save-route lives in the calc footer, disabled until a calc result exists
    await expect.element(screen.getByRole("button", { name: /save route/i })).toBeDisabled();
    // back to admin
    await expect.element(screen.getByRole("link", { name: /Admin/ })).toBeInTheDocument();
  });
});
