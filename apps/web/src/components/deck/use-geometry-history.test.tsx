import { act, type ReactNode } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, describe, expect, it } from "vitest";
import { FormProvider, type UseFormReturn, useForm } from "react-hook-form";
import { useGeometryHistory } from "./use-geometry-history";

// See use-deck-edit-rhf.test.tsx — this project has no @testing-library/react,
// so hand-roll the minimal renderHook (result ref + wrapper + act).
(globalThis as unknown as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

function renderHook<T>(
  callback: () => T,
  options: { wrapper: (props: { children: ReactNode }) => ReactNode },
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
    root.render(<Wrapper>{<Probe />}</Wrapper>);
  });
  return {
    result,
    // biome-ignore lint/style/noNonNullAssertion: assigned synchronously in act
    unmount: () => act(() => root!.unmount()),
  };
}

type Form = { geometry: GeoJSON.Geometry | null };
const polyA: GeoJSON.Polygon = { type: "Polygon", coordinates: [[[0, 0], [1, 0], [1, 1], [0, 0]]] };
const polyB: GeoJSON.Polygon = { type: "Polygon", coordinates: [[[0, 0], [2, 0], [2, 2], [0, 0]]] };

function mount() {
  let form!: UseFormReturn<Form>;
  const rendered = renderHook(() => useGeometryHistory("geometry"), {
    wrapper: ({ children }) => {
      form = useForm<Form>({ defaultValues: { geometry: null } });
      return <FormProvider {...form}>{children}</FormProvider>;
    },
  });
  return { ...rendered, getForm: () => form };
}

let mounted: { unmount: () => void } | null = null;
afterEach(() => {
  mounted?.unmount();
  mounted = null;
});

describe("useGeometryHistory", () => {
  it("records changes and undoes/redoes them", () => {
    const rendered = mount();
    mounted = rendered;
    const { result, getForm } = rendered;

    expect(result.current.canUndo).toBe(false);

    act(() => getForm().setValue("geometry", polyA));
    expect(result.current.canUndo).toBe(true);
    expect(result.current.canRedo).toBe(false);

    act(() => getForm().setValue("geometry", polyB));

    act(() => result.current.undo());
    expect(getForm().getValues("geometry")).toEqual(polyA);
    expect(result.current.canRedo).toBe(true);

    act(() => result.current.undo());
    expect(getForm().getValues("geometry")).toBeNull();
    expect(result.current.canUndo).toBe(false);

    act(() => result.current.redo());
    expect(getForm().getValues("geometry")).toEqual(polyA);
    expect(result.current.canUndo).toBe(true);
  });

  it("a fresh edit after undo clears the redo stack", () => {
    const rendered = mount();
    mounted = rendered;
    const { result, getForm } = rendered;

    act(() => getForm().setValue("geometry", polyA));
    act(() => result.current.undo());
    expect(result.current.canRedo).toBe(true);

    act(() => getForm().setValue("geometry", polyB));
    expect(result.current.canRedo).toBe(false);
    expect(result.current.canUndo).toBe(true);
  });
});
