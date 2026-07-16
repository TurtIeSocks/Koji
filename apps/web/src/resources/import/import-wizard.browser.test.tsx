import { describe, expect, it } from "vitest";
import { render } from "vitest-browser-react";
import { AdminContext } from "@/components/admin";
import { testDataProvider } from "shadmin-core";
import type { AuthProvider } from "shadmin-core";
import { ImportWizard } from "./import-wizard";

const auth: AuthProvider = {
  login: async () => undefined,
  logout: async () => undefined,
  checkAuth: async () => undefined,
  checkError: async () => undefined,
  getPermissions: async () => "admin",
  canAccess: async () => true,
};

const wrap = (node: React.ReactNode) => (
  <div style={{ width: "1024px", display: "flex", flexDirection: "column" }}>
    <AdminContext dataProvider={testDataProvider({})} authProvider={auth}>
      {/* Simulate the Layout's flex parent: flex flex-1 flex-col from layout.tsx:91 */}
      <div className="flex flex-1 flex-col">
        {node}
      </div>
    </AdminContext>
  </div>
);

describe("ImportWizard shell", () => {
  it("renders the stepper starting on Source", async () => {
    const screen = render(wrap(<ImportWizard />));
    await expect.element(screen.getByText("Source")).toBeVisible();
    await expect.element(screen.getByText("Review")).toBeVisible();
    // Step 0 panel = the Source step (its paste field is present).
    await expect.element(screen.getByLabelText(/paste geojson/i)).toBeVisible();
  });

  it("fills the available width (regression: mx-auto shrink-wrap to ~400px)", async () => {
    const screen = render(wrap(<ImportWizard />));
    await expect.element(screen.getByText("Import")).toBeVisible();
    const root = screen.container.querySelector(".max-w-5xl") as HTMLElement;
    expect(root).not.toBeNull();
    // Viewport in the browser runner is >= 1024px wide; without w-full the root
    // shrink-wraps to the Stepper (~400px). With w-full it engages max-w-5xl.
    expect(root.getBoundingClientRect().width).toBeGreaterThan(900);
  });
});
