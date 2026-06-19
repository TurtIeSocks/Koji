import { describe, expect, it } from "vitest";
import { render } from "vitest-browser-react";
import App from "@/App";

describe("Admin shell", () => {
  it("renders the login form when unauthenticated", async () => {
    const screen = render(<App />);
    await expect.element(screen.getByLabelText(/password/i)).toBeVisible();
  });
});
