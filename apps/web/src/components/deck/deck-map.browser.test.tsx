// Tailwind's utility classes (position/z-index) are only compiled into a real
// stylesheet when this global CSS entrypoint is imported — the "browser"
// vitest project has no shared setupFiles, so a plain className string does
// NOT visually apply without this. The expand overlay test below actually
// clicks through real CSS stacking (button above the deck.gl canvas), so it
// needs the real rules loaded, not just the class names present on the DOM.
import "@/index.css";
import type { ComponentProps } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { userEvent } from "@vitest/browser/context";
import { render } from "vitest-browser-react";
import type DeckGL from "@deck.gl/react";

// `getTooltip` only paints DOM (`.deck-tooltip`) on a real pointer hover,
// which isn't reliable to trigger in the harness — so assert the wiring
// instead: mock `@deck.gl/react`'s default export to capture the props
// DeckMap passes it, mirroring how deck-geojson-input.test.tsx stubs
// "./deck-map" itself to capture the `layers` prop it's given.
const { capturedProps } = vi.hoisted(() => ({
  capturedProps: { current: {} as ComponentProps<typeof DeckGL> },
}));
vi.mock("@deck.gl/react", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@deck.gl/react")>();
  return {
    ...actual,
    default: (props: ComponentProps<typeof DeckGL>) => {
      capturedProps.current = props;
      const Real = actual.default;
      return <Real {...props} />;
    },
  };
});

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

  it("forwards a provided getTooltip straight through to DeckGL", async () => {
    const getTooltip = () => ({ text: "hello" });
    const screen = render(
      <DeckMap layers={[]} height={300} getTooltip={getTooltip} />,
    );
    await expect.element(screen.getByTestId("deck-map")).toBeInTheDocument();
    expect(capturedProps.current.getTooltip).toBe(getTooltip);
  });

  it("leaves getTooltip undefined on DeckGL when the prop is omitted", async () => {
    const screen = render(<DeckMap layers={[]} height={300} />);
    await expect.element(screen.getByTestId("deck-map")).toBeInTheDocument();
    expect(capturedProps.current.getTooltip).toBeUndefined();
  });
});

// A map that mounts inside a `display:none` (0x0) container — e.g. a
// TabbedForm tab that isn't active yet — must not fit at a fallback size.
// It defers the fit until the container first reports a real, non-zero
// rect (via ResizeObserver), which for a hidden container is whenever it
// actually becomes visible.
describe("DeckMap deferred fit (hidden containers)", () => {
  const fenceWithBounds: GeoJSON.Polygon = {
    type: "Polygon",
    coordinates: [
      [
        [-10, -10],
        [10, -10],
        [10, 10],
        [-10, 10],
        [-10, -10],
      ],
    ],
  };

  beforeEach(() => {
    // Reset the capture between tests — otherwise a hidden map that never
    // re-renders would read a stale `initialViewState` left over from a
    // previous test's render.
    capturedProps.current = {} as ComponentProps<typeof DeckGL>;
  });

  it("does not render DeckGL while the container is hidden", async () => {
    const screen = render(
      <div data-testid="visibility-wrapper" style={{ display: "none" }}>
        <DeckMap layers={[]} fitBounds={fenceWithBounds} />
      </div>,
    );
    await expect
      .element(screen.getByTestId("visibility-wrapper"))
      .toBeInTheDocument();
    // Give a (buggy) synchronous fallback-size fit a moment to happen, then
    // assert it didn't — the fit stays deferred, so DeckGL never mounts and
    // `capturedProps` is never written.
    await new Promise((resolve) => setTimeout(resolve, 50));
    expect(capturedProps.current.initialViewState).toBeUndefined();
  });

  it("fits and renders DeckGL once the hidden container becomes visible", async () => {
    const screen = render(
      <div data-testid="visibility-wrapper" style={{ display: "none" }}>
        <DeckMap layers={[]} fitBounds={fenceWithBounds} />
      </div>,
    );
    const wrapper = screen.getByTestId("visibility-wrapper");
    await expect.element(wrapper).toBeInTheDocument();
    expect(capturedProps.current.initialViewState).toBeUndefined();

    (wrapper.element() as HTMLElement).style.display = "block";

    await vi.waitFor(() => {
      expect(capturedProps.current.initialViewState).toBeDefined();
    });
  });

  it("still fits immediately for a DeckMap that is visible at mount", async () => {
    const screen = render(<DeckMap layers={[]} fitBounds={fenceWithBounds} />);
    await expect.element(screen.getByTestId("deck-map")).toBeInTheDocument();
    await vi.waitFor(() => {
      expect(capturedProps.current.initialViewState).toBeDefined();
    });
  });
});
