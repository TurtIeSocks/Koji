import {
  TabbedForm,
  ReferenceArrayInput,
  AutocompleteArrayInput,
} from "@/components/admin";
import type { EditProps } from "@/components/admin/views/edit";
import { EditLive } from "@/components/realtime";
import { GeofenceMap } from "@/components/deck";
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
    <TabbedForm>
      <TabbedForm.Tab label="Details">
        <GeofenceEditFields />
      </TabbedForm.Tab>
      <TabbedForm.Tab label="Map" contentClassName="p-0">
        <GeofenceMap height="calc(100dvh - 16rem)" />
      </TabbedForm.Tab>
    </TabbedForm>
  </EditLive>
);
