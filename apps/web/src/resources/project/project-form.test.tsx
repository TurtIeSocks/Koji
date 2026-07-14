import { act, type ReactNode } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import { FormProvider, useForm, type UseFormReturn } from "react-hook-form";
import { AdminContext } from "@/components/admin";
import { ResourceContextProvider, testDataProvider } from "shadmin-core";
import type { AuthProvider } from "shadmin-core";

// `ProjectGeofencesMap` mounts the full deck.gl/MapLibre WebGL stack — not
// viable under jsdom. Stub it so this stays a fast unit test (`bun run test
// project-form`), not the browser provider. The wrapper under test
// (`ProjectFormMap`) lives in project-create.tsx itself (not in this mocked
// module), so it stays exercised for real.
vi.mock("@/components/deck/project-geofences-map", () => ({
  ProjectGeofencesMap: ({ ids }: { ids: (number | string)[] }) => (
    <div data-testid="pmap">{ids.join(",")}</div>
  ),
}));

import { ProjectFormFields } from "@/resources/project/project-create";

(globalThis as unknown as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const stubDataProvider = {
  ...testDataProvider({
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    getList: async () => ({ data: [] as any, total: 0 }),
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    getMany: async () => ({ data: [] as any }),
  }),
  subscribe: () => () => undefined,
};

const stubAuthProvider: AuthProvider = {
  login: async () => undefined,
  logout: async () => undefined,
  checkAuth: async () => undefined,
  checkError: async () => undefined,
  getPermissions: async () => "admin",
  canAccess: async () => true,
};

// Hand-rolled render (see project-show.test.tsx for why): mounts
// `ProjectFormFields` inside a real RHF `FormProvider` + `AdminContext` (the
// `ReferenceArrayInput`/`AutocompleteArrayInput` geofences field needs a
// dataProvider + resource context to resolve). Captures the live `form` API
// so tests can drive field changes and assert `ProjectFormMap` reactivity.
function renderProjectFormFields(defaultValues: Record<string, unknown>) {
  const container = document.createElement("div");
  document.body.appendChild(container);
  let root: Root;
  let formApi: UseFormReturn | null = null;

  function Harness() {
    const form = useForm({ defaultValues });
    formApi = form;
    return (
      <AdminContext dataProvider={stubDataProvider} authProvider={stubAuthProvider}>
        <ResourceContextProvider value="project">
          <FormProvider {...form}>
            <ProjectFormFields />
          </FormProvider>
        </ResourceContextProvider>
      </AdminContext>
    );
  }

  act(() => {
    root = createRoot(container);
    root.render(<Harness /> as ReactNode);
  });

  return {
    container,
    setGeofences: (ids: (number | string)[]) =>
      act(() => {
        formApi?.setValue("geofences", ids, { shouldDirty: true });
      }),
    unmount: () => act(() => root.unmount()),
  };
}

let mounted: ReturnType<typeof renderProjectFormFields> | null = null;
afterEach(() => {
  mounted?.unmount();
  mounted = null;
});

async function waitFor(assertion: () => void) {
  await act(async () => {
    await vi.waitFor(assertion, { timeout: 2000, interval: 20 });
  });
}

function pmapText(container: HTMLElement) {
  return container.querySelector('[data-testid="pmap"]')?.textContent;
}

describe("ProjectFormFields map", () => {
  it("renders ProjectFormMap fed by the geofences field's initial value", async () => {
    mounted = renderProjectFormFields({ geofences: [5] });
    await waitFor(() => expect(pmapText(mounted!.container)).toBe("5"));
  });

  it("updates reactively when the geofences field value changes", async () => {
    mounted = renderProjectFormFields({ geofences: [5] });
    await waitFor(() => expect(pmapText(mounted!.container)).toBe("5"));
    mounted.setGeofences([7, 8]);
    await waitFor(() => expect(pmapText(mounted!.container)).toBe("7,8"));
  });
});
