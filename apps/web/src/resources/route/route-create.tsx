import {
  Create,
  SimpleForm,
  TextInput,
  SelectInput,
  ReferenceInput,
} from "@/components/admin";
import { MultiPointInput } from "@/components/leaflet";
import { ROUTE_MODES, DEFAULT_TILE_URL } from "@/lib/constants";
import { required } from "ra-core";
// required is used for TextInput name validation

export const RouteFormFields = () => (
  <>
    <TextInput source="name" validate={required()} />
    <TextInput source="description" />
    <SelectInput source="mode" choices={[...ROUTE_MODES]} defaultValue="unset" />
    <ReferenceInput source="geofence_id" reference="geofence" />
    <MultiPointInput source="geometry" tileUrl={DEFAULT_TILE_URL} height={400} />
  </>
);

export const RouteCreate = () => (
  <Create>
    <SimpleForm>
      <RouteFormFields />
    </SimpleForm>
  </Create>
);
