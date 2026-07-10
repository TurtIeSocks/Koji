import {
  SimpleForm,
  ReferenceArrayInput,
  AutocompleteArrayInput,
} from "@/components/admin";
import type { EditProps } from "@/components/admin/views/edit";
import { EditLive } from "@/components/realtime";
import { DeckGeoJsonInput } from "@/components/deck";
import { GeofenceFormFields } from "./geofence-create";

const GeofenceEditFields = () => (
  <>
    <GeofenceFormFields />
    <DeckGeoJsonInput source="geometry" label="Geometry" height={400} />
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
