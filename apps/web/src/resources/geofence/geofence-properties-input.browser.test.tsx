import { describe, expect, it } from "vitest";
import { render } from "vitest-browser-react";
import { AdminContext } from "@/components/admin";
import { Form, ResourceContextProvider, testDataProvider } from "shadmin-core";
import type { AuthProvider } from "shadmin-core";
import { GeofencePropertiesInput } from "./geofence-properties-input";

const PROPERTIES = [
  { id: 1, name: "is_event", category: "boolean", default_value: null },
  { id: 2, name: "spawn_json", category: "object", default_value: null },
];

const RECORD = {
  id: 9,
  name: "Fence",
  mode: "unset",
  properties: [
    { id: 100, geofence_id: 9, property_id: 1, name: "is_event", category: "boolean", value: true },
  ],
};

const stubDataProvider = {
  ...testDataProvider({
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    getList: async () => ({ data: PROPERTIES as any, total: PROPERTIES.length }),
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    getMany: async () => ({ data: PROPERTIES as any }),
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    getOne: async () => ({ data: PROPERTIES[0] as any }),
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

const wrap = (node: React.ReactNode, record?: object) => (
  <AdminContext dataProvider={stubDataProvider} authProvider={stubAuthProvider}>
    <ResourceContextProvider value="geofence">
      {/* eslint-disable-next-line @typescript-eslint/no-explicit-any */}
      <Form record={record as any}>{node}</Form>
    </ResourceContextProvider>
  </AdminContext>
);

describe("GeofencePropertiesInput", () => {
  it("renders the array input with an add control", async () => {
    const screen = render(wrap(<GeofencePropertiesInput />));
    // SimpleFormIterator's add button carries the `button-add-properties` class.
    await expect
      .element(screen.container.querySelector(".button-add-properties"))
      .toBeInTheDocument();
  });

  it("hydrates an existing boolean property row with a boolean value widget", async () => {
    const screen = render(wrap(<GeofencePropertiesInput />, RECORD));
    // The property selector resolves property_id=1 → shows its name.
    await expect.element(screen.getByText("is_event")).toBeInTheDocument();
    // The value widget is typed by the looked-up category. categoryById is built
    // from an async useGetList, so the boolean widget appears only after the list
    // resolves — use a retrying locator (getByRole), NOT an eager querySelector,
    // so the poll spans the resolve. This proves the category lookup actually
    // drives the widget (the whole point of the row), not just the selector.
    await expect
      .element(screen.getByRole("switch").or(screen.getByRole("checkbox")))
      .toBeInTheDocument();
  });
});
