import { SimpleForm } from "@/components/admin";
import { EditLive } from "@/components/realtime";
import { RouteFormFields } from "./route-create";
import type { EditProps } from "@/components/admin/views/edit";

export const RouteEdit = (props: Pick<EditProps, "id">) => (
  <EditLive {...props}>
    <SimpleForm>
      <RouteFormFields />
    </SimpleForm>
  </EditLive>
);
