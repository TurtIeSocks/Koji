import { describe, expect, it } from "vitest";
import { render } from "vitest-browser-react";
import { AdminContext } from "@/components/admin";
import { RecordContextProvider, ResourceContextProvider, testDataProvider } from "shadmin-core";
import type { AuthProvider } from "shadmin-core";
import { ProjectShow } from "@/resources/project/project-show";

// biome-ignore lint/suspicious/noExplicitAny: test-local capture of ra-core's getManyReference params
let captured: any;

const stubDataProvider = {
  ...testDataProvider({
    getList: async () => ({
      data: [{ id: 1, name: "ReactMap reload", url: "http://rm/reload", mode: "ping", active: true, project_id: 10 }] as any,
      total: 1,
    }),
    getManyReference: async (_resource, params) => {
      captured = params;
      return {
        data: [{ id: 1, name: "ReactMap reload", url: "http://rm/reload", mode: "ping", active: true, project_id: 10 }] as any,
        total: 1,
      };
    },
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
    // Guards the ra-core→dataProvider wiring: ReferenceManyField must pass
    // target="project_id" and id=10 through to getManyReference so the
    // dataProvider can translate them into a `?project=10` filter. Without
    // this, the fixed-return stub above would pass even if the section
    // silently listed every webhook in the system (see data-provider.ts
    // getManyReference override).
    expect(captured?.target).toBe("project_id");
    expect(captured?.id).toBe(10);
  });
});
