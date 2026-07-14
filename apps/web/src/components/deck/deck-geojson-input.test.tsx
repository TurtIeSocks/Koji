import { act, type ReactNode } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import { FormProvider, useForm } from "react-hook-form";
import type { Layer } from "@deck.gl/core";

// `DeckMap` mounts a real DeckGL/MapLibre WebGL canvas — not viable under
// jsdom. Stub it to capture the `layers` prop instead, mirroring the
// hand-rolled renderHook approach in use-deck-edit-rhf.test.tsx (this
// project has no @testing-library/react dependency).
const { capturedLayers, capturedGetTooltip } = vi.hoisted(() => ({
  capturedLayers: { current: [] as Layer[] },
  // biome-ignore lint/suspicious/noExplicitAny: test-local capture of an arbitrary getTooltip prop
  capturedGetTooltip: { current: undefined as any },
}));
vi.mock("./deck-map", () => ({
  DeckMap: (props: {
    layers: Layer[];
    children?: ReactNode;
    // biome-ignore lint/suspicious/noExplicitAny: test-local capture of an arbitrary getTooltip prop
    getTooltip?: any;
  }) => {
    capturedLayers.current = props.layers;
    capturedGetTooltip.current = props.getTooltip;
    return props.children ?? null;
  },
}));

import { DeckGeoJsonInput } from "./deck-geojson-input";

(globalThis as unknown as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const poly: GeoJSON.Polygon = {
  type: "Polygon",
  coordinates: [
    [
      [0, 0],
      [1, 0],
      [1, 1],
      [0, 1],
      [0, 0],
    ],
  ],
};

function wrapper(defaultValues: Record<string, unknown>) {
  return ({ children }: { children: ReactNode }) => {
    const form = useForm({ defaultValues });
    return <FormProvider {...form}>{children}</FormProvider>;
  };
}

function renderWithForm(
  defaultValues: Record<string, unknown>,
  // biome-ignore lint/suspicious/noExplicitAny: test-local passthrough of arbitrary DeckGeoJsonInput props
  props: { disabled?: boolean; getTooltip?: any } = {},
) {
  const container = document.createElement("div");
  document.body.appendChild(container);
  let root: Root;
  const Wrapper = wrapper(defaultValues);
  act(() => {
    root = createRoot(container);
    root.render(
      <Wrapper>
        <DeckGeoJsonInput source="geometry" {...props} />
      </Wrapper>,
    );
  });
  return { unmount: () => act(() => root.unmount()) };
}

let mounted: { unmount: () => void } | null = null;
afterEach(() => {
  mounted?.unmount();
  mounted = null;
  capturedLayers.current = [];
  capturedGetTooltip.current = undefined;
});

describe("DeckGeoJsonInput", () => {
  it("auto-enters modify (editable layer) when a geometry is already present", () => {
    // An existing shape is editable on load without clicking a toolbar button.
    mounted = renderWithForm({ geometry: poly });
    expect(
      capturedLayers.current.some(
        (l) => l.id.startsWith("edit-") && l.id !== "edit-static",
      ),
    ).toBe(true);
    expect(capturedLayers.current.some((l) => l.id === "edit-static")).toBe(false);
  });

  it("stays read-only (static layer, no editable layer) when disabled", () => {
    // `disabled` overrides the auto-modify default.
    mounted = renderWithForm({ geometry: poly }, { disabled: true });
    expect(capturedLayers.current.some((l) => l.id === "edit-static")).toBe(true);
    expect(capturedLayers.current.some((l) => l.id === "edit")).toBe(false);
  });

  it("renders nothing extra when there is no existing value", () => {
    mounted = renderWithForm({ geometry: null });
    expect(capturedLayers.current).toHaveLength(0);
  });

  it("forwards getTooltip straight through to the inner DeckMap", () => {
    const getTooltip = () => ({ text: "hello" });
    mounted = renderWithForm({ geometry: poly }, { getTooltip });
    expect(capturedGetTooltip.current).toBe(getTooltip);
  });

  it("leaves getTooltip undefined on DeckMap when the prop is omitted", () => {
    mounted = renderWithForm({ geometry: poly });
    expect(capturedGetTooltip.current).toBeUndefined();
  });
});
