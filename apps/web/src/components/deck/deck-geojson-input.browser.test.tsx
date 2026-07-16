// Tailwind's utility classes (position/z-index) are only compiled into a real
// stylesheet when this global CSS entrypoint is imported — the "browser"
// vitest project has no shared setupFiles, so a toolbar button's `.click()`
// wouldn't land over the deck.gl canvas without it (established in the
// DeckMap expand-button test).
import "@/index.css";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { UseDeckEditRHFReturn } from "./use-deck-edit-rhf";

// Spy on useDeckEditRHF, delegating to the REAL hook by default (so the
// pre-existing tests below keep exercising real hook behavior unchanged) —
// only the hint test(s) at the bottom override the return value.
//
// Why: reaching "modify mode, 2 features, nothing selected" through real
// hydration isn't possible. The hook's pre-existing hydrate-time autoInit
// (use-deck-edit-rhf.ts: "with an existing shape, default straight into
// modify + select the first part") unconditionally selects index 0 the
// moment ANY non-empty geometry first hydrates, regardless of feature count —
// so seeding `record` with a multi-feature geometry lands already-selected,
// not empty. Reaching empty-selection-with-features any other way needs a
// real draw or a real deselect click on the deck.gl canvas, and this file's
// own "draw-toolbar buttons are type=button" test already documents that
// synthetic pointer events don't land on deck's canvas wrapper in headless.
// Driving the hook's return value directly isolates exactly what Task 12
// Step 5 changed — the hint's render condition in deck-geojson-input.tsx —
// while `enterMode`'s own auto-select logic is covered by
// use-deck-edit-rhf.test.tsx.
const { useDeckEditRHFMock, hookHolder } = vi.hoisted(() => ({
  useDeckEditRHFMock: vi.fn(),
  hookHolder: {} as { real?: (...args: never[]) => UseDeckEditRHFReturn },
}));
vi.mock("./use-deck-edit-rhf", async (importOriginal) => {
  const actual = await importOriginal<typeof import("./use-deck-edit-rhf")>();
  hookHolder.real = actual.useDeckEditRHF as (...args: never[]) => UseDeckEditRHFReturn;
  useDeckEditRHFMock.mockImplementation(hookHolder.real);
  return { ...actual, useDeckEditRHF: useDeckEditRHFMock };
});

import { render } from "vitest-browser-react";
import { AdminContext, SimpleForm } from "@/components/admin";
import { testDataProvider } from "shadmin-core";
import { DeckGeoJsonInput } from "./deck-geojson-input";

const TWO_FEATURE_DRAFT: GeoJSON.FeatureCollection = {
  type: "FeatureCollection",
  features: [
    {
      type: "Feature",
      geometry: {
        type: "Polygon",
        coordinates: [[[0, 0], [1, 0], [1, 1], [0, 1], [0, 0]]],
      },
      properties: {},
    },
    {
      type: "Feature",
      geometry: {
        type: "Polygon",
        coordinates: [[[10, 10], [11, 10], [11, 11], [10, 11], [10, 10]]],
      },
      properties: {},
    },
  ],
};

afterEach(() => {
  // Restore the real-hook delegate so a hint-test override never leaks into
  // a later test in this file. hookHolder.real is set once, synchronously,
  // by the vi.mock factory above — always populated before any test runs.
  if (hookHolder.real) useDeckEditRHFMock.mockImplementation(hookHolder.real);
});

describe("DeckGeoJsonInput", () => {
  it("renders the edit map + draw toolbar inside a form", async () => {
    const screen = render(
      <AdminContext dataProvider={testDataProvider()}>
        <SimpleForm onSubmit={() => {}}>
          <DeckGeoJsonInput source="geometry" label="Geometry" />
        </SimpleForm>
      </AdminContext>,
    );
    await expect.element(screen.getByTestId("deck-map")).toBeInTheDocument();
    await expect.element(screen.getByRole("button", { name: /polygon/i })).toBeInTheDocument();
  });

  it("exposes the full toolset, with split/cut-hole gated until a shape is selected", async () => {
    const screen = render(
      <AdminContext dataProvider={testDataProvider()}>
        <SimpleForm onSubmit={() => {}}>
          <DeckGeoJsonInput source="geometry" label="Geometry" />
        </SimpleForm>
      </AdminContext>,
    );
    for (const name of [/polygon/i, /rectangle/i, /circle/i, /modify/i, /move/i, /split/i, /cut hole/i]) {
      await expect.element(screen.getByRole("button", { name })).toBeInTheDocument();
    }
    // No geometry here → nothing selected → the shape-ops that draw onto a
    // selection are disabled.
    await expect.element(screen.getByRole("button", { name: /split/i })).toBeDisabled();
    await expect.element(screen.getByRole("button", { name: /cut hole/i })).toBeDisabled();
  });

  it("draw-toolbar buttons are type=button so a click never submits the form", async () => {
    // Regression: shadcn <Button> spreads props with no default `type`, so inside
    // a SimpleForm a toolbar click defaulted to type="submit" → submitted the form
    // and navigated away. The buttons must carry type="button". (Asserting the
    // attribute rather than clicking: deck's canvas wrapper intercepts synthetic
    // pointer events in headless, though the toolbar is clickable in a real tab.)
    const screen = render(
      <AdminContext dataProvider={testDataProvider()}>
        <SimpleForm onSubmit={() => {}}>
          <DeckGeoJsonInput source="geometry" label="Geometry" />
        </SimpleForm>
      </AdminContext>,
    );
    await expect
      .element(screen.getByRole("button", { name: /polygon/i }))
      .toHaveAttribute("type", "button");
  });

  it("shows Done/Cancel while drawing a polygon; Done disabled with <3 vertices", async () => {
    const screen = render(
      <AdminContext dataProvider={testDataProvider()}>
        <SimpleForm onSubmit={() => {}}>
          <DeckGeoJsonInput source="geometry" />
        </SimpleForm>
      </AdminContext>,
    );
    // Not drawing → no Done button.
    expect(screen.container.querySelector('[aria-label="Finish drawing"]')).toBeNull();
    await screen.getByRole("button", { name: "Polygon" }).click();
    await expect
      .element(screen.getByRole("button", { name: "Finish drawing" }))
      .toBeInTheDocument();
    await expect.element(screen.getByRole("button", { name: "Finish drawing" })).toBeDisabled();
    await expect
      .element(screen.getByRole("button", { name: "Cancel drawing" }))
      .toBeInTheDocument();
  });

  it("allowedModes restricts the toolbar buttons", async () => {
    const screen = render(
      <AdminContext dataProvider={testDataProvider()}>
        <SimpleForm onSubmit={() => {}}>
          <DeckGeoJsonInput source="geometry" allowedModes={["drawPolygon", "modify"]} />
        </SimpleForm>
      </AdminContext>,
    );
    await expect.element(screen.getByRole("button", { name: /polygon/i })).toBeInTheDocument();
    await expect.element(screen.getByRole("button", { name: /modify/i })).toBeInTheDocument();
    expect(screen.container.querySelector('[aria-label="Move"]')).toBeNull();
    expect(screen.container.querySelector('[aria-label="Split"]')).toBeNull();
  });

  it("shows the select-a-shape hint in modify mode with nothing selected", async () => {
    // Two features, mode already "modify", nothing selected — the state a
    // user reaches after clicking Modify with an ambiguous (multi-shape)
    // draft (Task 12's enterMode deliberately leaves selection empty there).
    useDeckEditRHFMock.mockReturnValue({
      draft: TWO_FEATURE_DRAFT,
      mode: "modify",
      setMode: vi.fn(),
      selectedIndexes: [],
      onEdit: vi.fn(),
      onSelect: vi.fn(),
      deleteSelected: vi.fn(),
      version: 0,
    } satisfies UseDeckEditRHFReturn);
    const screen = render(
      <AdminContext dataProvider={testDataProvider()}>
        <SimpleForm onSubmit={() => {}}>
          <DeckGeoJsonInput source="geometry" />
        </SimpleForm>
      </AdminContext>,
    );
    await expect
      .element(screen.getByText("Click a shape to edit its points"))
      .toBeVisible();
  });

  it("hides the hint once a shape is selected", async () => {
    // Regression: the hint must not linger once enterMode (or a click)
    // resolves a selection — same 2-feature draft, this time pre-selected.
    useDeckEditRHFMock.mockReturnValue({
      draft: TWO_FEATURE_DRAFT,
      mode: "modify",
      setMode: vi.fn(),
      selectedIndexes: [0],
      onEdit: vi.fn(),
      onSelect: vi.fn(),
      deleteSelected: vi.fn(),
      version: 0,
    } satisfies UseDeckEditRHFReturn);
    const screen = render(
      <AdminContext dataProvider={testDataProvider()}>
        <SimpleForm onSubmit={() => {}}>
          <DeckGeoJsonInput source="geometry" />
        </SimpleForm>
      </AdminContext>,
    );
    await expect.element(screen.getByTestId("deck-map")).toBeInTheDocument();
    expect(screen.getByText("Click a shape to edit its points").query()).toBeNull();
  });
});
