import {
  Create,
  SimpleForm,
  TextInput,
  ReferenceArrayInput,
  AutocompleteArrayInput,
} from "@/components/admin";
import { required } from "ra-core";
import { useWatch } from "react-hook-form";
import { ProjectGeofencesMap } from "@/components/deck/project-geofences-map";

/** Thin wrapper feeding the live-edited `geofences` field value into
 *  `ProjectGeofencesMap`, so the map stays reactive to the
 *  AutocompleteArrayInput selection above it. Lives here (not in
 *  project-geofences-map.tsx) so tests can mock `ProjectGeofencesMap` alone
 *  while exercising this wiring for real. */
const ProjectFormMap = () => {
  const ids = (useWatch({ name: "geofences" }) as (number | string)[] | undefined) ?? [];
  return <ProjectGeofencesMap ids={ids} />;
};

export const ProjectFormFields = () => (
  <>
    <TextInput source="name" validate={required()} />
    <TextInput source="description" label="Description" multiline />
    <ReferenceArrayInput source="geofences" reference="geofence">
      <AutocompleteArrayInput />
    </ReferenceArrayInput>
    <ProjectFormMap />
  </>
);

export const ProjectCreate = () => (
  <Create>
    <SimpleForm>
      <ProjectFormFields />
    </SimpleForm>
  </Create>
);
