import { describe, expect, it } from "vitest";
import { render } from "vitest-browser-react";
import { AdminContext } from "@/components/admin";
import { ResourceContextProvider, testDataProvider } from "shadmin-core";
import type { AuthProvider } from "shadmin-core";
import { Create, SimpleForm } from "@/components/admin";
import { SelectInput } from "@/components/admin";
import { PropertyValueInput } from "@/components/inputs/property-value-input";

const stubDataProvider = {
  ...testDataProvider({
    getList: async () => ({ data: [] as any, total: 0 }),
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

const wrap = (node: React.ReactNode) => (
  <AdminContext dataProvider={stubDataProvider} authProvider={stubAuthProvider}>
    <ResourceContextProvider value="property">{node}</ResourceContextProvider>
  </AdminContext>
);

/** Render a node inside a minimal RHF form with AdminContext. */
const renderInForm = (
  node: React.ReactNode,
  options?: { defaultValues?: Record<string, unknown> },
) =>
  render(
    wrap(
      <Create>
        <SimpleForm defaultValues={options?.defaultValues ?? {}}>
          {node}
        </SimpleForm>
      </Create>,
    ),
  );

describe("PropertyValueInput", () => {
  it("renders a TextInput by default (string category)", async () => {
    const screen = render(
      wrap(
        <Create>
          <SimpleForm defaultValues={{ category: "string" }}>
            <SelectInput
              source="category"
              choices={[{ id: "string", name: "String" }, { id: "boolean", name: "Boolean" }]}
            />
            <PropertyValueInput source="default_value" />
          </SimpleForm>
        </Create>,
      ),
    );
    // Default category=string → TextInput
    await expect.element(screen.getByLabelText(/default.value/i)).toBeVisible();
  });

  it("renders a BooleanInput when category is boolean", async () => {
    const screen = render(
      wrap(
        <Create>
          <SimpleForm defaultValues={{ category: "boolean" }}>
            <SelectInput
              source="category"
              choices={[{ id: "boolean", name: "Boolean" }]}
            />
            <PropertyValueInput source="default_value" />
          </SimpleForm>
        </Create>,
      ),
    );
    // category=boolean → BooleanInput (a switch)
    await expect.element(screen.getByRole("switch")).toBeVisible();
  });

  // --- New cases: explicit category prop ---

  it("renders BooleanInput when category prop is 'boolean' (explicit, no sibling)", async () => {
    const screen = renderInForm(
      <PropertyValueInput source="value" category="boolean" label="Value" />,
    );
    await expect.element(screen.getByLabelText(/value/i)).toBeVisible();
    // BooleanInput renders a switch/checkbox role
    await expect
      .element(screen.getByRole("switch").or(screen.getByRole("checkbox")))
      .toBeVisible();
  });

  it("renders a Monaco JSON editor when category prop is 'object'", async () => {
    const screen = renderInForm(
      <PropertyValueInput source="value" category="object" label="Value" />,
    );
    // MonacoJsonInput shows the helperText naming the JSON category.
    await expect.element(screen.getByText(/must be a json object/i)).toBeVisible();
  });

  it("uses the explicit category over the watched sibling field", async () => {
    // Sibling category field says "number" but the prop says "color" → prop wins.
    const screen = renderInForm(
      <PropertyValueInput source="value" category="color" />,
      { defaultValues: { category: "number", value: "#ff0000" } },
    );
    // ColorInput renders a native color input.
    await expect
      .element(screen.container.querySelector('input[type="color"]'))
      .toBeInTheDocument();
  });
});
