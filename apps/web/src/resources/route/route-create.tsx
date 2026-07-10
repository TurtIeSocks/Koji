import {
  Create,
  SimpleForm,
  TextInput,
  SelectInput,
  ReferenceInput,
} from "@/components/admin";
import { RouteMap } from "@/components/deck";
import { ROUTE_MODES } from "@/lib/constants";
import { required } from "ra-core";

/** Shared route form body (create + edit): metadata + the deck calc workbench.
 *  Routes are calc results over a geofence — `<RouteMap>` reactively loads the
 *  fence from `geofence_id`, runs the calc, and writes the result to `geometry`.
 *  (No Leaflet: the map is the deck `<RouteMap>` on both create and edit.) */
export const RouteFormFields = () => (
  <>
    <TextInput source="name" validate={required()} />
    <TextInput source="description" />
    <SelectInput source="mode" choices={[...ROUTE_MODES]} defaultValue="unset" />
    <ReferenceInput source="geofence_id" reference="geofence" />
    <RouteMap />
  </>
);

export const RouteCreate = () => (
  <Create>
    {/* Wider than the default max-w-lg — the calc workbench map needs the room. */}
    <SimpleForm className="max-w-4xl">
      <RouteFormFields />
    </SimpleForm>
  </Create>
);
