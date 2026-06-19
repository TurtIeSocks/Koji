import { SimpleForm } from "@/components/admin";
import type { EditProps } from "@/components/admin/views/edit";
import { EditLive } from "@/components/realtime";
import { GeofenceFormFields } from "./geofence-create";

export const GeofenceEdit = (props: Pick<EditProps, "id">) => (
  <EditLive {...props}>
    <SimpleForm>
      <GeofenceFormFields />
    </SimpleForm>
  </EditLive>
);
