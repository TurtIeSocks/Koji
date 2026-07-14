import {
  Create,
  TabbedForm,
  TextInput,
  SelectInput,
  ReferenceInput,
} from "@/components/admin";
import { GeofenceMap } from "@/components/deck";
import { GEOFENCE_MODES } from "@/lib/constants";
import { required } from "ra-core";
import { GeofencePropertiesInput } from "@/resources/geofence/geofence-properties-input";

// Metadata fields shared by Create + Edit. The geometry map (<GeofenceMap>) is a
// sibling element in both — Edit adds a projects input after it, so it stays out
// of this component. Both pages use the deck map; there is no Leaflet anywhere.
export const GeofenceFormFields = () => (
  <>
    <TextInput source="name" validate={required()} />
    <SelectInput source="mode" choices={[...GEOFENCE_MODES]} defaultValue="unset" />
    <ReferenceInput source="parent" reference="geofence" />
    <GeofencePropertiesInput />
  </>
);

export const GeofenceCreate = () => (
  <Create>
    <TabbedForm>
      <TabbedForm.Tab label="Details">
        <GeofenceFormFields />
      </TabbedForm.Tab>
      <TabbedForm.Tab label="Map" contentClassName="p-0">
        <GeofenceMap height="calc(100dvh - 16rem)" />
      </TabbedForm.Tab>
    </TabbedForm>
  </Create>
);
