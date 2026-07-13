// Tailwind's utility classes (position/z-index) are only compiled into a real
// stylesheet when this global CSS entrypoint is imported — the "browser"
// vitest project has no shared setupFiles, so a plain className string does
// NOT visually apply without this. The expand overlay test below actually
// clicks through real CSS stacking (button above the deck.gl canvas), so it
// needs the real rules loaded, not just the class names present on the DOM.
import "@/index.css";
import { describe, expect, it } from "vitest";
import { userEvent } from "@vitest/browser/context";
import { render } from "vitest-browser-react";
import { DeckMap } from "./deck-map";

describe("DeckMap", () => {
  it("mounts the deck + maplibre root without throwing", async () => {
    const screen = render(<DeckMap layers={[]} height={300} />);
    await expect.element(screen.getByTestId("deck-map")).toBeInTheDocument();
  });

  it("renders no expand button when `expandable` is not set", async () => {
    const screen = render(<DeckMap layers={[]} height={300} />);
    await expect.element(screen.getByTestId("deck-map")).toBeInTheDocument();
    await expect
      .element(screen.getByRole("button", { name: /expand|full/i }))
      .not.toBeInTheDocument();
  });

  it("toggles a fixed viewport overlay on expand click and collapses on Escape", async () => {
    const screen = render(<DeckMap layers={[]} height={300} expandable />);
    const container = screen.getByTestId("deck-map");
    await expect.element(container).toBeInTheDocument();
    await expect.element(container).not.toHaveClass(/fixed/);

    const expandBtn = screen.getByRole("button", { name: /expand|full/i });
    await expect.element(expandBtn).toBeInTheDocument();
    await expandBtn.click();
    await expect.element(container).toHaveClass(/fixed/);
    await expect.element(container).toHaveClass("inset-0");

    await userEvent.keyboard("{Escape}");
    await expect.element(container).not.toHaveClass(/fixed/);
  });
});
