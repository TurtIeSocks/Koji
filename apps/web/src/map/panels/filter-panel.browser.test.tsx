import { render } from "vitest-browser-react";
import { beforeEach, expect, test } from "vitest";
import { FilterPanel } from "@/map/panels/filter-panel";
import { useMapUIStore } from "@/map/stores/map-ui-store";

beforeEach(() => useMapUIStore.setState(useMapUIStore.getInitialState()));

test("selecting a TTH option updates the store", async () => {
  const screen = render(<FilterPanel />);
  // shadcn Select: open via combobox role, then pick by text
  await screen.getByRole("combobox").click();
  // Use first() to resolve the "Known" vs "Unknown" strict-mode ambiguity:
  // the options are ordered All → Known → Unknown so index 0 of filtered results is "Known".
  await screen.getByText("Known", { exact: true }).first().click();
  expect(useMapUIStore.getState().filters.tth).toBe("Known");
});
