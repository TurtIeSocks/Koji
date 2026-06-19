import {
  SimpleForm,
  ReferenceArrayInput,
  AutocompleteArrayInput,
} from "@/components/admin";
import type { EditProps } from "@/components/admin/views/edit";
import { EditLive } from "@/components/realtime";
import { GeofenceFormFields } from "./geofence-create";

const GeofenceEditFields = () => (
  <>
    <GeofenceFormFields />
    <ReferenceArrayInput source="projects" reference="project">
      <AutocompleteArrayInput />
    </ReferenceArrayInput>
  </>
);

export const GeofenceEdit = (props: Pick<EditProps, "id">) => (
  <EditLive {...props}>
    <SimpleForm>
      <GeofenceEditFields />
    </SimpleForm>
  </EditLive>
);
