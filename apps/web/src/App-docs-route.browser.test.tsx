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

// ApiDocs mounts Scalar, which fetches /api/v2/openapi.yaml on render — no
// backend here, so stub it to a sentinel (same precedent as the /map test
// stubbing MapPlayground).
vi.mock("@/components/docs/api-docs", () => ({
  ApiDocs: () => <div data-testid="api-docs" />,
}));

import App from "@/App";

describe("/docs route", () => {
  it("renders the API docs full-bleed, gated behind auth", async () => {
    const screen = render(
      <MemoryRouter initialEntries={["/docs"]}>
        <App />
      </MemoryRouter>,
    );
    await expect.element(screen.getByTestId("api-docs")).toBeInTheDocument();
  });
});
