import { render } from "vitest-browser-react";
import { beforeEach, expect, test } from "vitest";
import { LayerDrawer } from "@/map/panels/layer-drawer";
import { useMapUIStore } from "@/map/stores/map-ui-store";

beforeEach(() => useMapUIStore.setState(useMapUIStore.getInitialState()));

test("clicking a layer toggle flips that layer in the store", async () => {
  const screen = render(<LayerDrawer />);
  expect(useMapUIStore.getState().layerVisibility.gyms).toBe(false);
  await screen.getByRole("switch", { name: /gyms/i }).click();
  expect(useMapUIStore.getState().layerVisibility.gyms).toBe(true);
});
