// Tailwind's utility classes (position/z-index) are only compiled into a real
// stylesheet when this global CSS entrypoint is imported — the "browser"
// vitest project has no shared setupFiles, so the toggle button's `.click()`
// wouldn't land over the deck.gl canvas without it (established in the
// DeckMap expand-button test).
import "@/index.css";
import { describe, expect, it, vi } from "vitest";
import { render } from "vitest-browser-react";
import { RecordContextProvider, testDataProvider } from "shadmin-core";
import { GeoJsonLayer } from "@deck.gl/layers";
import { AdminContext } from "@/components/admin";
import { DeckGeoJsonField } from "./deck-geojson-field";

const poly: GeoJSON.Polygon = { type: "Polygon", coordinates: [[[0,0],[1,0],[1,1],[0,1],[0,0]]] };

const SPAWNPOINTS: [number, number][] = [
  [4.905, 51.905],
  [4.915, 51.915],
];

vi.mock("@/map/data/use-markers", () => ({
  useMarkers: vi.fn(
    (
      category: string,
      _area: unknown,
      _bounds: unknown,
      _lastSeen: number,
      enabled: boolean,
    ) => ({
      data: !enabled ? undefined : category === "spawnpoint" ? SPAWNPOINTS : [],
    }),
  ),
}));

function renderField(props: Partial<React.ComponentProps<typeof DeckGeoJsonField>> = {}) {
  return render(
    <AdminContext dataProvider={testDataProvider()}>
      <RecordContextProvider value={{ id: 1, geometry: poly }}>
        <DeckGeoJsonField source="geometry" {...props} />
      </RecordContextProvider>
    </AdminContext>,
  );
}

describe("DeckGeoJsonField", () => {
  it("renders the map when the record has geometry", async () => {
    const screen = renderField();
    await expect.element(screen.getByTestId("deck-map")).toBeInTheDocument();
  });

  it("shows empty text when geometry is missing", async () => {
    const screen = render(
      <AdminContext dataProvider={testDataProvider()}>
        <RecordContextProvider value={{ id: 1 }}>
          <DeckGeoJsonField source="geometry" emptyText="No geometry" />
        </RecordContextProvider>
      </AdminContext>,
    );
    await expect.element(screen.getByText("No geometry")).toBeVisible();
  });

  it("renders a marker toggle + expand button when markerMode is set, and the toggle flips its label on click", async () => {
    const screen = renderField({ markerMode: "pokemon", markerArea: poly, expandable: true });

    const toggle = screen.getByRole("button", { name: "Show Spawnpoints" });
    await expect.element(toggle).toBeInTheDocument();

    const expandBtn = screen.getByRole("button", { name: /expand/i });
    await expect.element(expandBtn).toBeInTheDocument();

    await toggle.click();
    await expect
      .element(screen.getByRole("button", { name: "Hide Spawnpoints" }))
      .toBeInTheDocument();
  });

  it("renders no marker toggle when markerMode is not set (back-compat)", async () => {
    const screen = renderField();
    await expect.element(screen.getByTestId("deck-map")).toBeInTheDocument();
    await expect.element(screen.getByRole("button", { name: /show/i })).not.toBeInTheDocument();
  });

  it("hides the marker toggle for an unset mode (no categories to show)", async () => {
    const screen = renderField({ markerMode: "unset", markerArea: poly });
    await expect.element(screen.getByTestId("deck-map")).toBeInTheDocument();
    await expect
      .element(screen.getByRole("button", { name: /show/i }))
      .not.toBeInTheDocument();
  });

  it("defaults height to 640 when height is omitted", async () => {
    const screen = renderField();
    const el = screen.getByTestId("deck-map").element() as HTMLElement;
    expect(el.style.height).toBe("640px");
  });

  it("renders extraControls (and accepts extraLayers) when provided", async () => {
    const extra = new GeoJsonLayer({
      id: "extra",
      data: { type: "Feature", properties: {}, geometry: poly } satisfies GeoJSON.Feature,
    });
    const screen = renderField({
      extraLayers: [extra],
      extraControls: <button type="button">Show Neighbors</button>,
    });
    await expect.element(screen.getByTestId("deck-map")).toBeInTheDocument();
    await expect
      .element(screen.getByRole("button", { name: "Show Neighbors" }))
      .toBeInTheDocument();
  });

  it("renders no extraControls when omitted (back-compat)", async () => {
    const screen = renderField();
    await expect.element(screen.getByTestId("deck-map")).toBeInTheDocument();
    await expect
      .element(screen.getByRole("button", { name: "Show Neighbors" }))
      .not.toBeInTheDocument();
  });
});
