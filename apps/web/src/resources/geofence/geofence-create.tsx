import {
  Create,
  SimpleForm,
  TextInput,
  SelectInput,
  ReferenceInput,
} from "@/components/admin";
import { MultiPolygonInput } from "@/components/leaflet";
import { GEOFENCE_MODES, DEFAULT_TILE_URL } from "@/lib/constants";
import { required } from "ra-core";

export const GeofenceFormFields = () => (
  <>
    <TextInput source="name" validate={required()} />
    <SelectInput source="mode" choices={[...GEOFENCE_MODES]} defaultValue="unset" />
    <ReferenceInput source="parent" reference="geofence" />
    <MultiPolygonInput source="geometry" tileUrl={DEFAULT_TILE_URL} height={400} />
  </>
);

export const GeofenceCreate = () => (
  <Create>
    <SimpleForm>
      <GeofenceFormFields />
    </SimpleForm>
  </Create>
);
