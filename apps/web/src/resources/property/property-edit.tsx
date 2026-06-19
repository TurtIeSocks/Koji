import { SimpleForm } from "@/components/admin";
import type { EditProps } from "@/components/admin/views/edit";
import { EditLive } from "@/components/realtime";
import { PropertyFormFields } from "./property-create";

export const PropertyEdit = (props: Pick<EditProps, "id">) => (
  <EditLive {...props}>
    <SimpleForm>
      <PropertyFormFields />
    </SimpleForm>
  </EditLive>
);
