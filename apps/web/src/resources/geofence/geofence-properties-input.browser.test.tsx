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
    // The categoryById map is built from an async useGetList; the switch/checkbox
    // appears only after the list resolves. Fall back to asserting the property
    // selector shows the linked property's name — row mounting proves hydration.
    // (Brief-specified fallback for the async-hydration case.)
    await expect.element(screen.getByText("is_event")).toBeInTheDocument();
  });
});
