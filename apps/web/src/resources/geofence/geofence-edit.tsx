import { SimpleForm } from "@/components/admin";
import { EditLive } from "@/components/realtime";
import { GeofenceFormFields } from "./geofence-create";

export const GeofenceEdit = () => (
  <EditLive>
    <SimpleForm>
      <GeofenceFormFields />
    </SimpleForm>
  </EditLive>
);
