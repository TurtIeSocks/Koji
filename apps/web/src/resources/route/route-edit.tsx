import { SimpleForm, TextInput, SelectInput, ReferenceInput } from "@/components/admin";
import { EditLive } from "@/components/realtime";
import { RouteMap } from "@/components/deck";
import { ROUTE_MODES } from "@/lib/constants";
import { required } from "ra-core";
import type { EditProps } from "@/components/admin/views/edit";

export const RouteEdit = (props: Pick<EditProps, "id">) => (
  <EditLive {...props}>
    <SimpleForm>
      <TextInput source="name" validate={required()} />
      <TextInput source="description" />
      <SelectInput source="mode" choices={[...ROUTE_MODES]} />
      <ReferenceInput source="geofence_id" reference="geofence" />
      <RouteMap />
    </SimpleForm>
  </EditLive>
);
