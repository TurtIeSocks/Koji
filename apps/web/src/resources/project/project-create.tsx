import {
  Create,
  TabbedForm,
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
export const ProjectFormMap = ({ height }: { height?: number | string }) => {
  const ids = (useWatch({ name: "geofences" }) as (number | string)[] | undefined) ?? [];
  return <ProjectGeofencesMap ids={ids} height={height} />;
};

/** Metadata fields shared by Create + Edit. The member map (`<ProjectFormMap>`)
 *  is a sibling element in both — it gets its own full-width tab, so it stays
 *  out of this component. */
export const ProjectMetaFields = () => (
  <>
    <TextInput source="name" validate={required()} />
    <TextInput source="description" label="Description" multiline />
    <ReferenceArrayInput source="geofences" reference="geofence">
      <AutocompleteArrayInput />
    </ReferenceArrayInput>
  </>
);

export const ProjectCreate = () => (
  <Create>
    <TabbedForm>
      <TabbedForm.Tab label="Details">
        <ProjectMetaFields />
      </TabbedForm.Tab>
      <TabbedForm.Tab label="Map" contentClassName="p-0">
        <ProjectFormMap height="calc(100dvh - 16rem)" />
      </TabbedForm.Tab>
    </TabbedForm>
  </Create>
);
