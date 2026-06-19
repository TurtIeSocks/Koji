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
});
