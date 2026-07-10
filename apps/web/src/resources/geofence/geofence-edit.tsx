import {
  SimpleForm,
  ReferenceArrayInput,
  AutocompleteArrayInput,
  Toolbar,
} from "@/components/admin";
import type { EditProps } from "@/components/admin/views/edit";
import { EditLive } from "@/components/realtime";
import { GeofenceMap } from "@/components/deck";
import { GeofenceFormFields } from "./geofence-create";

const GeofenceEditFields = () => (
  <>
    <GeofenceFormFields />
    <GeofenceMap />
    <ReferenceArrayInput source="projects" reference="project">
      <AutocompleteArrayInput />
    </ReferenceArrayInput>
  </>
);

export const GeofenceEdit = (props: Pick<EditProps, "id">) => (
  <EditLive {...props}>
    {/* Toolbar = Save + Delete (the default SimpleForm toolbar is Cancel+Save,
        no delete). Gives per-record geofence deletion; the list also has bulk. */}
    <SimpleForm toolbar={<Toolbar />}>
      <GeofenceEditFields />
    </SimpleForm>
  </EditLive>
);
