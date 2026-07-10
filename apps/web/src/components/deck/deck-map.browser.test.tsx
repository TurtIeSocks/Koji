import { describe, expect, it } from "vitest";
import { render } from "vitest-browser-react";
import { DeckMap } from "./deck-map";

describe("DeckMap", () => {
  it("mounts the deck + maplibre root without throwing", async () => {
    const screen = render(<DeckMap layers={[]} height={300} />);
    await expect.element(screen.getByTestId("deck-map")).toBeInTheDocument();
  });
});
