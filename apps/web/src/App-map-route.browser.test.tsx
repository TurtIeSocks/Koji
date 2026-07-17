import { describe, expect, it, vi } from "vitest";
import { render } from "vitest-browser-react";
import { MemoryRouter } from "react-router";

// Authenticated gate calls authProvider.checkAuth() via a real HTTP fetch —
// no backend in this test, so stub the whole provider to resolve as logged in.
vi.mock("@/api/live/auth-provider", () => ({
  authProvider: {
    login: vi.fn(),
    logout: vi.fn(),
    checkAuth: vi.fn().mockResolvedValue(undefined),
    checkError: vi.fn(),
    getPermissions: vi.fn().mockResolvedValue("admin"),
    canAccess: vi.fn().mockResolvedValue(true),
  },
}));

// MapPlayground pulls in useMarkers/useCalc (real query + @api calc) — stub it,
// same precedent as the old test stubbing DeckCanvas/MapIndex. Other exports from
// this barrel are used elsewhere in the import graph, so pass them through.
vi.mock("@/components/deck", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@/components/deck")>();
  return { ...actual, MapPlayground: () => <div data-testid="deck-map" /> };
});

import App from "@/App";

describe("/map route", () => {
  it("renders the playground full-bleed, gated behind auth", async () => {
    const screen = render(
      <MemoryRouter initialEntries={["/map"]}>
        <App />
      </MemoryRouter>,
    );
    await expect.element(screen.getByTestId("deck-map")).toBeInTheDocument();
  });
});
