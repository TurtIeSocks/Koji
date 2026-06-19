import { SimpleForm } from "@/components/admin";
import type { EditProps } from "@/components/admin/views/edit";
import { EditLive } from "@/components/realtime";
import { ProjectFormFields } from "./project-create";

export const ProjectEdit = (props: Pick<EditProps, "id">) => (
  <EditLive {...props}>
    <SimpleForm>
      <ProjectFormFields />
    </SimpleForm>
  </EditLive>
);
