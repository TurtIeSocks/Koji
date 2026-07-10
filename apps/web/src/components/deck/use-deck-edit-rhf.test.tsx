import { act, type ReactNode } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import { FormProvider, useForm } from "react-hook-form";
import { useDeckEditRHF } from "./use-deck-edit-rhf";

// Silence React's "not configured to support act(...)" warning — normally set
// by `@testing-library/react`'s own environment setup, which this project
// doesn't depend on (see the hand-rolled `renderHook` below).
(globalThis as unknown as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

// `@testing-library/react` isn't a dependency of this project (only
// `@testing-library/jest-dom` is, for matchers) — hand-roll the minimal
// `renderHook` shape (result ref + wrapper + act) that the rest of this test
// needs, backed by `react-dom/client` + React 19's own `act`.
function renderHook<T>(
  callback: () => T,
  options: { wrapper?: (props: { children: ReactNode }) => ReactNode } = {},
) {
  const result: { current: T } = { current: undefined as unknown as T };
  function Probe() {
    result.current = callback();
    return null;
  }
  const container = document.createElement("div");
  document.body.appendChild(container);
  let root: Root;
  act(() => {
    root = createRoot(container);
    const Wrapper = options.wrapper;
    root.render(Wrapper ? <Wrapper>{<Probe />}</Wrapper> : <Probe />);
  });
  return {
    result,
    unmount: () => act(() => root.unmount()),
  };
}

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

let mounted: { unmount: () => void } | null = null;
afterEach(() => {
  mounted?.unmount();
  mounted = null;
});

describe("useDeckEditRHF", () => {
  it("hydrates the draft from the form value", () => {
    const rendered = renderHook(() => useDeckEditRHF({ source: "geometry" }), {
      wrapper: wrapper({ geometry: poly }),
    });
    mounted = rendered;
    const { result } = rendered;
    expect(result.current.draft.features).toHaveLength(1);
    expect(result.current.draft.features[0].geometry).toEqual(poly);
  });

  it("commits an edited geometry back to the form value", () => {
    const spy = vi.fn();
    const rendered = renderHook(() => useDeckEditRHF({ source: "geometry" }), {
      wrapper: ({ children }) => {
        const form = useForm({ defaultValues: { geometry: poly } });
        vi.spyOn(form, "setValue").mockImplementation((...a: unknown[]) => {
          spy(...a);
        });
        return <FormProvider {...form}>{children}</FormProvider>;
      },
    });
    mounted = rendered;
    const { result } = rendered;

    act(() => {
      result.current.onEdit({
        updatedData: {
          type: "FeatureCollection",
          features: [
            {
              type: "Feature",
              geometry: {
                type: "Polygon",
                coordinates: [
                  [
                    [0, 0],
                    [2, 0],
                    [2, 2],
                    [0, 2],
                    [0, 0],
                  ],
                ],
              },
              properties: {},
            },
          ],
        },
        editType: "movePosition",
      });
    });

    expect(spy).toHaveBeenCalledWith(
      "geometry",
      expect.objectContaining({ type: "Polygon" }),
      expect.anything(),
    );
  });
});
