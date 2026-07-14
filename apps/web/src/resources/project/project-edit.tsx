import { TabbedForm } from "@/components/admin";
import type { EditProps } from "@/components/admin/views/edit";
import { EditLive } from "@/components/realtime";
import { ProjectMetaFields, ProjectFormMap } from "./project-create";

export const ProjectEdit = (props: Pick<EditProps, "id">) => (
  <EditLive {...props}>
    <TabbedForm>
      <TabbedForm.Tab label="Details">
        <ProjectMetaFields />
      </TabbedForm.Tab>
      <TabbedForm.Tab label="Map" contentClassName="p-0">
        <ProjectFormMap height="calc(100dvh - 16rem)" />
      </TabbedForm.Tab>
    </TabbedForm>
  </EditLive>
);
