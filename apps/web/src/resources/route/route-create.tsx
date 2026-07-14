import {
  Create,
  TabbedForm,
  TextInput,
  SelectInput,
  ReferenceInput,
} from "@/components/admin";
import { RouteMap } from "@/components/deck";
import { ROUTE_MODES } from "@/lib/constants";
import { required } from "ra-core";

/** Metadata fields shared by Create + Edit. The calc workbench (`<RouteMap>`)
 *  is a sibling element in both — it gets its own full-width tab, so it stays
 *  out of this component. */
export const RouteMetaFields = () => (
  <>
    <TextInput source="name" validate={required()} />
    <TextInput source="description" />
    <SelectInput source="mode" choices={[...ROUTE_MODES]} defaultValue="unset" />
    <ReferenceInput source="geofence_id" reference="geofence" />
  </>
);

export const RouteCreate = () => (
  <Create>
    <TabbedForm>
      <TabbedForm.Tab label="Details">
        <RouteMetaFields />
      </TabbedForm.Tab>
      <TabbedForm.Tab label="Map" contentClassName="p-0">
        <RouteMap height="calc(100dvh - 16rem)" />
      </TabbedForm.Tab>
    </TabbedForm>
  </Create>
);
