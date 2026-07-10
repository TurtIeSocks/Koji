import { describe, expect, it, vi } from "vitest";
import { render } from "vitest-browser-react";
import { MemoryRouter } from "react-router";

// Authenticated gate calls authProvider.checkAuth() via a real HTTP fetch —
// no backend in this test, so stub the whole provider to resolve as logged in.
vi.mock("@/auth-provider", () => ({
  authProvider: {
    login: vi.fn(),
    logout: vi.fn(),
    checkAuth: vi.fn().mockResolvedValue(undefined),
    checkError: vi.fn(),
    getPermissions: vi.fn().mockResolvedValue("admin"),
    canAccess: vi.fn().mockResolvedValue(true),
  },
}));

// MapIndex pulls in useGeoFeatures (ra-core useGetList -> live dataProvider) —
// stub it, same precedent as the old map-route test stubbing DeckCanvas. Other
// exports from this barrel (GeofenceMap, RouteMap, ...) are used elsewhere in
// the app's import graph, so pass them through untouched.
vi.mock("@/components/deck", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@/components/deck")>();
  return { ...actual, MapIndex: () => <div data-testid="deck-map" /> };
});

import App from "@/App";

describe("/map route", () => {
  it("renders the read-only map index full-bleed with a back-to-admin link, no draw toolbar or calc panel", async () => {
    const screen = render(
      <MemoryRouter initialEntries={["/map"]}>
        <App />
      </MemoryRouter>,
    );
    await expect.element(screen.getByTestId("deck-map")).toBeInTheDocument();
    await expect.element(screen.getByRole("link", { name: /Admin/ })).toBeInTheDocument();
    expect(screen.getByTestId("draw-toolbar").query()).toBeNull();
    expect(screen.getByTestId("calc-panel").query()).toBeNull();
  });
});
