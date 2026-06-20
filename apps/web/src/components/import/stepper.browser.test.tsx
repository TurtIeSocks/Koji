import { describe, expect, it } from "vitest";
import { render } from "vitest-browser-react";
import { Stepper } from "./stepper";

describe("Stepper", () => {
  it("renders each step label and marks the active one", async () => {
    const screen = render(
      <Stepper steps={["Source", "Map & Name", "Assign", "Review"]} active={2} />,
    );
    await expect.element(screen.getByText("Source")).toBeVisible();
    await expect.element(screen.getByText("Assign")).toBeVisible();
    await expect
      .element(screen.container.querySelector('[aria-current="step"]'))
      .toBeInTheDocument();
  });
});
