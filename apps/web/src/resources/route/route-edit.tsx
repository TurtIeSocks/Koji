import { SimpleForm } from "@/components/admin";
import type { EditProps } from "@/components/admin/views/edit";
import { EditLive } from "@/components/realtime";
import { RouteFormFields } from "./route-create";

export const RouteEdit = (props: Pick<EditProps, "id">) => (
  <EditLive {...props}>
    <SimpleForm>
      <RouteFormFields />
    </SimpleForm>
  </EditLive>
);
