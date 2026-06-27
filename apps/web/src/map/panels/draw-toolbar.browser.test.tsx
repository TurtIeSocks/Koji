import { render } from "vitest-browser-react";
import { beforeEach, expect, test } from "vitest";
import { DrawToolbar } from "@/map/panels/draw-toolbar";
import { useMapUIStore } from "@/map/stores/map-ui-store";

beforeEach(() => useMapUIStore.setState(useMapUIStore.getInitialState()));

test("clicking Polygon sets the draw mode; Cancel clears the draft", async () => {
  const screen = render(<DrawToolbar />);
  await screen.getByRole("button", { name: /polygon/i }).click();
  expect(useMapUIStore.getState().drawMode).toBe("drawPolygon");
  await screen.getByRole("button", { name: /cancel/i }).click();
  expect(useMapUIStore.getState().drawMode).toBe("none");
});
