import { describe, expect, it } from "vitest";
import { render } from "vitest-browser-react";
import { userEvent } from "@vitest/browser/context";
import { FormProvider, useForm, useWatch } from "react-hook-form";
import { AdminContext } from "@/components/admin";
import { testDataProvider, ResourceContextProvider } from "shadmin-core";
import type { AuthProvider } from "shadmin-core";
import { AssignStep } from "./assign-step";

// ── Auth harness (mirrors import-wizard.browser.test.tsx) ─────────────────────
const auth: AuthProvider = {
  login: async () => undefined,
  logout: async () => undefined,
  checkAuth: async () => undefined,
  checkError: async () => undefined,
  getPermissions: async () => "admin",
  canAccess: async () => true,
};

// Data provider: override getList + getMany so ReferenceInputs return empty lists.
const dataProvider = {
  ...testDataProvider({
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    getList: async () => ({ data: [] as any[], total: 0 }),
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    getMany: async () => ({ data: [] as any[] }),
  }),
};

// Two features: one Polygon (→ geofence), one MultiPoint (→ route).
const TWO_FEATURES = [
  {
    type: "Feature",
    geometry: { type: "Polygon", coordinates: [] },
    properties: { label: "Alpha" },
    name: "",
  },
  {
    type: "Feature",
    geometry: { type: "MultiPoint", coordinates: [] },
    properties: { label: "Beta" },
    name: "",
  },
];

// ResourceContextProvider mirrors import-wizard.tsx (value="geofence");
// SimpleFormIteratorItem requires a resource in context.
function Wrapper({ children }: { children: React.ReactNode }) {
  const methods = useForm({
    defaultValues: {
      features: TWO_FEATURES,
      _name_prop: "label",
      _name_template: "{name}",
    },
  });
  return (
    <AdminContext dataProvider={dataProvider} authProvider={auth}>
      <ResourceContextProvider value="geofence">
        <FormProvider {...methods}>{children}</FormProvider>
      </ResourceContextProvider>
    </AdminContext>
  );
}

// Reads the live RHF form state so a test can assert each row's mode by index
// (not by counting display spans, which can't distinguish row 0 from row 1).
function ModesProbe() {
  const features = useWatch({ name: "features" }) as
    | Array<{ mode?: string }>
    | undefined;
  return (
    <output data-testid="row-modes">
      {(features ?? []).map((f) => f?.mode ?? "").join(",")}
    </output>
  );
}

describe("AssignStep", () => {
  it("renders 2 rows, each with a name input and mode label", async () => {
    const screen = render(
      <Wrapper>
        <AssignStep />
      </Wrapper>,
    );

    // The iterator header label is visible.
    await expect.element(screen.getByText("Features")).toBeVisible();

    // Name seeding: two Name inputs rendered, one per row.
    // TextInput uses a label "Name" — there are 2, use container querySelectorAll.
    const nameInputs = screen.container.querySelectorAll<HTMLInputElement>(
      "input[type=text]",
    );
    // Expect at least 2 text inputs (one per row's "Name" field).
    expect(nameInputs.length).toBeGreaterThanOrEqual(2);
    await expect.element(nameInputs[0]).toBeVisible();
    await expect.element(nameInputs[1]).toBeVisible();

    // Each row has a "Mode" label text — verify at least 2 in the DOM.
    const modeLabels = screen.container.querySelectorAll("label, span");
    const modeLabelTexts = Array.from(modeLabels).filter(
      (el) => el.textContent?.trim() === "Mode",
    );
    expect(modeLabelTexts.length).toBeGreaterThanOrEqual(2);
  });

  it("bulk Apply mode to all sets both rows' mode", async () => {
    const screen = render(
      <Wrapper>
        <AssignStep />
        <ModesProbe />
      </Wrapper>,
    );

    // Wait for name inputs to render (confirms rows are mounted).
    await expect.element(screen.getByText("Features")).toBeVisible();

    // Both rows start unset (the probe reflects the live RHF features array).
    await expect
      .element(screen.getByTestId("row-modes"))
      .not.toHaveTextContent("pokemon");

    // Change the bulk mode native select to "pokemon".
    const nativeSelect = screen.container.querySelector<HTMLSelectElement>(
      'select[aria-label="Bulk mode"]',
    )!;
    expect(nativeSelect).toBeTruthy();
    await userEvent.selectOptions(nativeSelect, "pokemon");

    // Click "Apply mode to all".
    await userEvent.click(
      screen.getByRole("button", { name: /apply mode to all/i }),
    );

    // Assert each row's mode independently via the live form state — both rows
    // must be "pokemon" (row-anchored; cannot false-pass on a stray display span).
    await expect
      .element(screen.getByTestId("row-modes"))
      .toHaveTextContent("pokemon,pokemon");
  });

  it("Polygon row shows Parent input; MultiPoint row shows Route parent input", async () => {
    const screen = render(
      <Wrapper>
        <AssignStep />
      </Wrapper>,
    );

    // Row 0 = Polygon → kind "geofence" → a label/span with text "Parent" is present.
    // Row 1 = MultiPoint → kind "route" → a label/span with text "Route parent" is present.
    // "Route parent" text is visible.
    await expect.element(screen.getByText("Route parent")).toBeVisible();

    // "Parent" text exists as standalone label (not just as substring of "Route parent").
    // Find an element whose exact text is "Parent".
    const parentEl = Array.from(
      screen.container.querySelectorAll<HTMLElement>("label, span, p, div"),
    ).find((el) => el.textContent?.trim() === "Parent");
    expect(parentEl).toBeTruthy();
    await expect.element(parentEl!).toBeVisible();
  });
});
