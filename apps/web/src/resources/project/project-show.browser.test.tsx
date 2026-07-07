import { describe, expect, it } from "vitest";
import { render } from "vitest-browser-react";
import { AdminContext } from "@/components/admin";
import { RecordContextProvider, ResourceContextProvider, testDataProvider } from "shadmin-core";
import type { AuthProvider } from "shadmin-core";
import { ProjectShow } from "@/resources/project/project-show";

const stubDataProvider = {
  ...testDataProvider({
    getList: async () => ({
      data: [{ id: 1, name: "ReactMap reload", url: "http://rm/reload", mode: "ping", active: true, project_id: 10 }] as any,
      total: 1,
    }),
    getManyReference: async () => ({
      data: [{ id: 1, name: "ReactMap reload", url: "http://rm/reload", mode: "ping", active: true, project_id: 10 }] as any,
      total: 1,
    }),
    getOne: async () => ({ data: { id: 10, name: "Proj-A" } as any }),
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

describe("ProjectShow webhooks section", () => {
  it("lists webhooks scoped to the project", async () => {
    const screen = render(
      <AdminContext dataProvider={stubDataProvider} authProvider={stubAuthProvider}>
        <ResourceContextProvider value="project">
          <RecordContextProvider value={{ id: 10, name: "Proj-A" }}>
            <ProjectShow id={10} />
          </RecordContextProvider>
        </ResourceContextProvider>
      </AdminContext>,
    );
    await expect.element(screen.getByText("ReactMap reload")).toBeVisible();
  });
});
